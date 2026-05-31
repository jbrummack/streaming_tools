use crate::download::{
    StreamingResultHandlerExt, progress::ProgressCounter, streamer::StreamingResponse,
};
pub struct LinesDownloader {
    reader: StreamingResponse,
    buffer: String,
}

impl LinesDownloader {
    pub async fn read_line<'a>(&'a mut self) -> Result<Option<&'a str>, crate::Error> {
        self.buffer.clear();
        let res = self
            .reader
            .read_line(&mut self.buffer)
            .await
            .handle_eof(self.reader.number_of_items())?
            .map(|_| self.buffer.as_ref());
        if self.buffer.is_empty() {
            Ok(None)
        } else {
            Ok(res)
        }
    }
    pub fn get_progress(&self) -> ProgressCounter {
        self.reader.get_counter()
    }
}

impl StreamingResponse {
    pub fn download_lines(self) -> LinesDownloader {
        LinesDownloader {
            reader: self,
            buffer: String::with_capacity(1024),
        }
    }
}
