use std::{
    pin::Pin,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering::Relaxed},
    },
    task::{Context, Poll},
};

use bytes::Bytes;
use futures::{Stream, TryStreamExt};
use http_body_util::BodyExt;
use hyper::body::{Body, Incoming};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio_util::io::StreamReader;

use crate::download::progress::ProgressCounter;

pub struct StreamingResponse {
    number_of_items: usize,
    expected_size: Option<usize>,
    already_downloaded: Arc<AtomicUsize>,
    pub(crate) reader: BufReader<Box<dyn tokio::io::AsyncRead + Send + Unpin>>,
}
struct MeteredStream<S> {
    already_downloaded: Arc<AtomicUsize>,
    inner: S,
}
impl<S> MeteredStream<S> {
    pub fn wrap(inner: S) -> Self {
        Self {
            already_downloaded: Arc::new(AtomicUsize::new(0)),
            inner,
        }
    }
    pub fn get_counter(&self) -> Arc<AtomicUsize> {
        self.already_downloaded.clone()
    }
}
impl<S, E> Stream for MeteredStream<S>
where
    S: Stream<Item = Result<Bytes, E>> + Unpin,
{
    type Item = Result<Bytes, E>;

    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        match Pin::new(&mut self.inner).poll_next(cx) {
            Poll::Ready(Some(Ok(chunk))) => {
                self.already_downloaded.fetch_add(chunk.len(), Relaxed);

                Poll::Ready(Some(Ok(chunk)))
            }

            other => other,
        }
    }
}

impl StreamingResponse {
    pub async fn read_u32(&mut self) -> std::io::Result<usize> {
        use tokio::io::AsyncReadExt;
        let r = self.reader.read_u32().await?;
        Ok(r as usize)
    }
    pub async fn read_varint(&mut self) -> std::io::Result<usize> {
        use tokio::io::AsyncReadExt;
        let mut result = 0usize;
        let mut shift = 0;
        loop {
            if shift >= 64 {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "varint too long",
                ));
            }
            let mut buf = [0u8; 1];
            self.reader.read_exact(&mut buf).await?;
            let byte = buf[0];
            result |= ((byte & 0x7F) as usize) << shift;

            if byte & 0x80 == 0 {
                return Ok(result);
            }
            shift += 7;
        }
    }
    pub fn number_of_items(&self) -> usize {
        self.number_of_items
    }
    pub async fn read_exact(&mut self, buffer: &mut [u8]) -> std::io::Result<()> {
        use tokio::io::AsyncReadExt;
        self.reader.read_exact(buffer).await?;
        self.number_of_items += 1;
        Ok(())
    }
    pub async fn read_line(&mut self, buffer: &mut String) -> std::io::Result<()> {
        self.reader.read_line(buffer).await?;
        self.number_of_items += 1;
        Ok(())
    }
    pub fn get_counter(&self) -> ProgressCounter {
        ProgressCounter {
            goal: self.expected_size,
            counter: self.already_downloaded.clone(),
        }
    }
    pub async fn new(body: Incoming) -> Self {
        let expected_size = body.size_hint().upper().map(|u| u as usize);
        let stream = body.into_data_stream();
        use async_compression::tokio::bufread::GzipDecoder;
        let stream = stream.into_stream();

        let metered = MeteredStream::wrap(stream);
        let already_downloaded = metered.get_counter();
        let reader = StreamReader::new(
            metered.map_err(|e| std::io::Error::new(std::io::ErrorKind::Other, e)),
        );
        let decoder = GzipDecoder::new(reader);
        let reader: BufReader<Box<dyn tokio::io::AsyncRead + Send + Unpin>> =
            BufReader::new(Box::new(decoder));
        Self {
            expected_size,
            already_downloaded,
            reader,
            number_of_items: 0,
        }
    }
}
