use bytes::Bytes;
use http_body_util::Full;
use hyper::{Method, Request};
use hyper_rustls::{HttpsConnector, HttpsConnectorBuilder};
use hyper_util::client::legacy::{Client, connect::HttpConnector};

use crate::download::{
    lines::LinesDownloader, proto::ProtoDownloader, streamer::StreamingResponse,
};

pub struct ClientWrapper<B> {
    client: Client<hyper_rustls::HttpsConnector<HttpConnector>, B>,
}
impl ClientWrapper<Full<Bytes>> {
    pub async fn download(&self, url: impl AsRef<str>) -> Result<StreamingResponse, crate::Error> {
        let request = Request::builder()
            .method(Method::GET)
            .uri(url.as_ref())
            .body(Full::new(Bytes::new()))?;
        let response = self.client.request(request).await?;
        let (metadata, body) = response.into_parts();
        let status = metadata.status;
        let reason = status.canonical_reason().unwrap_or("no reason given");
        if status.is_server_error() {
            return Err(crate::Error::ServerStatus(reason));
        }
        if status.is_client_error() {
            return Err(crate::Error::ClientStatus(reason));
        }

        Ok(StreamingResponse::new(body).await)
    }
}
pub fn config() -> HttpsConnector<HttpConnector> {
    let mut root_store = rustls::RootCertStore::empty();
    root_store.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    let https = HttpsConnectorBuilder::new()
        .with_tls_config(config)
        //.with_native_roots()? // Or .with_webpki_roots()
        .https_only()
        .enable_http2()
        //.enable_http2()
        .build();

    https
}

pub struct Downloader(ClientWrapper<Full<Bytes>>);
impl Downloader {
    pub fn new() -> Self {
        Self(ClientWrapper {
            client: Client::builder(hyper_util::rt::TokioExecutor::new()).build(config()),
        })
    }
    pub async fn download_lines(
        &self,
        url: impl AsRef<str>,
    ) -> Result<LinesDownloader, crate::Error> {
        let response = self.0.download(url).await?;
        Ok(response.download_lines())
    }
    pub async fn download_protos(
        &self,
        url: impl AsRef<str>,
    ) -> Result<ProtoDownloader, crate::Error> {
        let response = self.0.download(url).await?;
        Ok(response.download_protos())
    }
}
/*impl Uploader {
    pub fn new() -> Self {
        let client = Client::builder(hyper_util::rt::TokioExecutor::new()).build(config());

        let (mut uploader, body) = create_gzip_upload();
        let req = Request::put("uri").body(body).unwrap();
        client.request(req).await.unwrap();
        todo!()
        /*Self(ClientWrapper {
            client: Client::builder(hyper_util::rt::TokioExecutor::new()).build(config()),
        })*/
    }
}*/
/*impl<B: Send> ClientWrapper<B> {
    pub fn downloader() -> Result<ClientWrapper<Full<Bytes>>, crate::Error> {
        ClientWrapper::new()
    }
    pub fn uploader() -> Result<ClientWrapper<StreamBody<ReaderStream<DuplexStream>>>, crate::Error>
    {
        Ok(ClientWrapper {
            client: Client::builder(hyper_util::rt::TokioExecutor::new()).build(Self::config()?),
        })
    }

}*/
