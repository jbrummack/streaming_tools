use http::StatusCode;

use crate::upload::Uploader;

pub struct ProtoUploader {
    buffer: Vec<u8>,
    inner: Uploader,
}

impl ProtoUploader {
    pub fn new(presigned_url: impl Into<String>) -> Self {
        Self {
            inner: Uploader::new(presigned_url),
            buffer: Vec::with_capacity(1024),
        }
    }
    pub async fn upload_proto32<T: prost::Message>(
        &mut self,
        value: &T,
    ) -> Result<(), crate::UploadError> {
        self.buffer.clear();

        let len = value.encoded_len().to_be_bytes();
        for byte in len {
            self.buffer.push(byte);
        }
        value.encode(&mut self.buffer)?;
        self.inner.write_chunk(&self.buffer).await
    }
    pub async fn upload_varint<T: prost::Message>(
        &mut self,
        value: &T,
    ) -> Result<(), crate::UploadError> {
        self.buffer.clear();

        value.encode_length_delimited(&mut self.buffer)?;
        self.inner.write_chunk(&self.buffer).await
    }
    pub async fn finish(self) -> Result<StatusCode, crate::UploadError> {
        self.inner.finish().await
    }
}
