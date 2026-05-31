use http::StatusCode;

use crate::upload::Uploader;

pub struct LinesUploader {
    inner: Uploader,
}

impl LinesUploader {
    pub fn new(presigned_url: impl Into<String>) -> Self {
        Self {
            inner: Uploader::new(presigned_url),
        }
    }
    ///Uploads text without '\n'
    pub async fn upload_text(&mut self, text: impl AsRef<str>) -> Result<(), crate::UploadError> {
        self.inner.write_chunk(text.as_ref().as_bytes()).await
    }
    pub async fn upload_line(&mut self, text: impl AsRef<str>) -> Result<(), crate::UploadError> {
        self.upload_text(text).await?;
        self.upload_text("\n").await?;
        Ok(())
    }
    pub async fn finish(self) -> Result<StatusCode, crate::UploadError> {
        self.inner.finish().await
    }
}
