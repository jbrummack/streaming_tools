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
/// A connector that trusts the system roots through `rustls-native-certs`.
pub fn config() -> HttpsConnector<HttpConnector> {
    let mut root_store = rustls::RootCertStore::empty();
    // OS trust store (Security.framework on Apple, system cacerts on Android/Linux)
    let native = rustls_native_certs::load_native_certs();
    for err in &native.errors {
        tracing::warn!("Failed to load a native root certificate: {err}");
    }
    let (_added, _ignored) = root_store.add_parsable_certificates(native.certs);
    let config = rustls::ClientConfig::builder()
        .with_root_certificates(root_store)
        .with_no_client_auth();
    config_with(config)
}
/// A connector with the caller's TLS configuration, for platforms where the roots cannot be loaded as certificates
/// (iOS: the caller passes a config that verifies through the system).
pub fn config_with(config: rustls::ClientConfig) -> HttpsConnector<HttpConnector> {
    let https = HttpsConnectorBuilder::new()
        .with_tls_config(config)
        .https_only()
        .enable_http2()
        //.enable_http2()
        .build();

    https
}

pub struct Downloader(ClientWrapper<Full<Bytes>>);
impl Downloader {
    pub fn new() -> Self {
        Self::with_connector(config())
    }
    pub fn with_tls_config(config: rustls::ClientConfig) -> Self {
        Self::with_connector(config_with(config))
    }
    fn with_connector(connector: HttpsConnector<HttpConnector>) -> Self {
        Self(ClientWrapper {
            client: Client::builder(hyper_util::rt::TokioExecutor::new()).build(connector),
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
