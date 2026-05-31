use crate::download::{
    StreamingResultHandlerExt, progress::ProgressCounter, streamer::StreamingResponse,
};

pub struct ProtoDownloader {
    reader: StreamingResponse,
    buffer: Vec<u8>,
}

impl ProtoDownloader {
    ///Reads varint, resizes its buffer, fills it and tries to decode then
    pub async fn read_proto_varint<T: prost::Message + Default>(
        &mut self,
    ) -> Result<Option<T>, crate::Error> {
        self.buffer.clear();
        let len = if let Some(len) = self
            .reader
            .read_varint()
            .await
            .handle_eof(self.reader.number_of_items())?
        {
            len
        } else {
            return Ok(None);
        };
        if len == 0 {
            return Ok(None);
        }
        self.buffer.resize(len, 0);
        self.reader.read_exact(&mut self.buffer).await?;

        let res = T::decode(self.buffer.as_ref())?;

        Ok(Some(res))
    }
    ///Reads u32, resizes its buffer, fills it and tries to decode then
    pub async fn read_proto_u32<T: prost::Message + Default>(
        &mut self,
    ) -> Result<Option<T>, crate::Error> {
        self.buffer.clear();
        let len = if let Some(len) = self
            .reader
            .read_u32()
            .await
            .handle_eof(self.reader.number_of_items())?
        {
            len
        } else {
            return Ok(None);
        };
        if len == 0 {
            return Ok(None);
        }
        self.buffer.resize(len, 0);
        self.reader.read_exact(&mut self.buffer).await?;

        let res = T::decode(self.buffer.as_ref())?;

        Ok(Some(res))
    }
    pub async fn test_proto32<'a>(&'a mut self) -> Result<Option<&'a [u8]>, crate::Error> {
        self.buffer.clear();
        let len = if let Some(len) = self
            .reader
            .read_u32()
            .await
            .handle_eof(self.reader.number_of_items())?
        {
            len
        } else {
            return Ok(None);
        };
        //println!("{len}");
        if len == 0 {
            return Ok(None);
        }
        self.buffer.resize(len, 0);
        self.reader.read_exact(&mut self.buffer).await?;

        let res = self.buffer.as_ref();

        Ok(Some(res))
    }
    pub fn get_progress(&self) -> ProgressCounter {
        self.reader.get_counter()
    }
}
impl StreamingResponse {
    pub fn download_protos(self) -> ProtoDownloader {
        ProtoDownloader {
            reader: self,
            buffer: Vec::with_capacity(1024),
        }
    }
}
