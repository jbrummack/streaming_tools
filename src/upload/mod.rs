pub mod lines;
pub mod proto;

use std::pin::Pin;
use std::task::{Context, Poll};

use async_compression::tokio::write::GzipEncoder;
use bytes::Bytes;

use hyper::body::Frame;
use hyper::{Method, Request, StatusCode};
use hyper_util::rt::TokioExecutor;
use tokio::io::{AsyncWriteExt, DuplexStream};
use tokio::sync::oneshot;

use crate::client::config;

/// Accepts `&[u8]` chunks, compresses them with gzip on the fly, and streams
/// the compressed bytes to a presigned S3 PUT URL.
///
/// # Usage
/// ```rust
/// let mut uploader = S3GzipUploader::new(presigned_url);
/// uploader.write_chunk(b"hello ").await?;
/// uploader.write_chunk(b"world").await?;
/// let status = uploader.finish().await?;
/// assert_eq!(status, 200);
/// ```
pub struct Uploader {
    /// GzipEncoder wraps the write half of the duplex pipe.
    encoder: GzipEncoder<DuplexStream>,
    /// Receives the HTTP status code once the upload task completes.
    result_rx: oneshot::Receiver<Result<StatusCode, UploadError>>,
}

#[derive(Debug, thiserror::Error)]
pub enum UploadError {
    #[error("hyper error: {0}")]
    Hyper(#[from] hyper::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] hyper::http::Error),
    #[error("S3 returned non-2xx status: {0}")]
    S3Status(StatusCode),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Encode error: {0}")]
    Encode(#[from] prost::EncodeError),
    #[error("TLS/connect error: {0}")]
    Connect(String),
}

impl Uploader {
    /// Spawn the upload task and return a handle you can write chunks into.
    pub fn new(presigned_url: impl Into<String>) -> Self {
        let presigned_url: String = presigned_url.into();
        // tokio::io::duplex gives us a pair of AsyncRead + AsyncWrite ends.
        // buf_size controls the in-memory backpressure window.
        let (write_half, read_half) = tokio::io::duplex(256 * 1024); // 256 KiB window

        // The encoder writes *compressed* bytes into `write_half`.
        let encoder = GzipEncoder::new(write_half);

        // Wrap the read half in a stream of Bytes so hyper can consume it.
        let body = DuplexReadBody::new(read_half);

        let (result_tx, result_rx) = oneshot::channel();

        // Spawn the upload on the background; it will block waiting for body
        // data produced by the encoder on the other side of the pipe.
        tokio::spawn(async move {
            let outcome = upload_body(presigned_url, body).await;
            let _ = result_tx.send(outcome);
        });

        Self { encoder, result_rx }
    }

    /// Feed a raw (uncompressed) chunk. Can be called many times.
    pub async fn write_chunk(&mut self, data: &[u8]) -> Result<(), UploadError> {
        self.encoder.write_all(data).await.map_err(UploadError::Io)
    }

    /// Flush + shut down the encoder, then await the HTTP response.
    /// Returns the HTTP status code on success.
    pub async fn finish(mut self) -> Result<StatusCode, UploadError> {
        // Flush remaining buffered data and write the gzip trailer.
        self.encoder.shutdown().await.map_err(UploadError::Io)?;
        // Drop the encoder → drops write_half → signals EOF to read_half.
        drop(self.encoder);

        // Wait for the upload task to finish and return its result.
        self.result_rx
            .await
            .map_err(|_| UploadError::Connect("upload task panicked".into()))?
    }
}

// ── DuplexReadBody ────────────────────────────────────────────────────────────

/// A hyper-compatible body that drains a `DuplexStream` (the read half).
struct DuplexReadBody {
    inner: tokio::io::ReadHalf<DuplexStream>,
}

impl DuplexReadBody {
    fn new(duplex: DuplexStream) -> Self {
        let (read_half, _) = tokio::io::split(duplex);
        Self { inner: read_half }
    }
}

impl hyper::body::Body for DuplexReadBody {
    type Data = Bytes;
    type Error = std::io::Error;

    fn poll_frame(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Result<Frame<Self::Data>, Self::Error>>> {
        use tokio::io::AsyncRead;

        let mut buf = [0u8; 16 * 1024]; // 16 KiB read buffer
        let mut read_buf = tokio::io::ReadBuf::new(&mut buf);

        match Pin::new(&mut self.inner).poll_read(cx, &mut read_buf) {
            Poll::Pending => Poll::Pending,
            Poll::Ready(Err(e)) => Poll::Ready(Some(Err(e))),
            Poll::Ready(Ok(())) => {
                let filled = read_buf.filled();
                if filled.is_empty() {
                    Poll::Ready(None) // EOF
                } else {
                    Poll::Ready(Some(Ok(Frame::data(Bytes::copy_from_slice(filled)))))
                }
            }
        }
    }
}

// ── HTTP upload task ──────────────────────────────────────────────────────────

async fn upload_body(url: String, body: DuplexReadBody) -> Result<StatusCode, UploadError> {
    let connector = config();
    let client = hyper_util::client::legacy::Client::builder(TokioExecutor::new()).build(connector);

    let uri: hyper::Uri = url
        .parse()
        .map_err(|e| UploadError::Connect(format!("{e}")))?;

    let request = Request::builder()
        .method(Method::PUT)
        .uri(uri)
        // S3 presigned URLs require Content-Encoding so it stores gzip.
        .header("Content-Encoding", "gzip")
        // content-type is whatever you're uploading; adjust as needed.
        .header("Content-Type", "application/octet-stream")
        .body(body)
        .map_err(UploadError::Http)?;

    let response = client
        .request(request)
        .await
        .map_err(|e| UploadError::Connect(e.to_string()))?;

    let status = response.status();
    if status.is_success() {
        Ok(status)
    } else {
        Err(UploadError::S3Status(status))
    }
}
