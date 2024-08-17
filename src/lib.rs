pub mod nkv;
pub mod notifier;
mod persist_value;
pub mod request_msg;

use crate::request_msg::*;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::TcpStream;

pub struct NatsClient {
    writer: BufWriter<tokio::net::tcp::OwnedWriteHalf>,
    reader: BufReader<tokio::net::tcp::OwnedReadHalf>,
}

impl NatsClient {
    pub async fn new(addr: &str) -> tokio::io::Result<Self> {
        let stream = TcpStream::connect(addr).await?;
        let (read, write) = stream.into_split();
        let writer = BufWriter::new(write);
        let reader = BufReader::new(read);

        Ok(Self { writer, reader })
    }

    pub async fn get(&mut self, key: String) -> tokio::io::Result<ServerResponse> {
        let req = ServerRequest::Get(BaseMessage { id: 0, key });
        self.send_request(&req).await
    }

    pub async fn put(&mut self, key: String, val: Box<[u8]>) -> tokio::io::Result<ServerResponse> {
        let req = ServerRequest::Put(PutMessage {
            base: BaseMessage { id: 0, key },
            value: val,
        });
        self.send_request(&req).await
    }

    pub async fn delete(&mut self, key: String) -> tokio::io::Result<ServerResponse> {
        let req = ServerRequest::Delete(BaseMessage { id: 0, key });
        self.send_request(&req).await
    }

    pub async fn subscribe(&mut self, key: String) -> tokio::io::Result<ServerResponse> {
        let req = ServerRequest::Subscribe(BaseMessage { id: 0, key });
        self.send_request(&req).await
    }

    async fn send_request(&mut self, request: &ServerRequest) -> tokio::io::Result<ServerResponse> {
        let req = serde_json::to_string(&request)?;
        self.writer.write_all(req.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;

        let mut response_buf = String::new();

        self.reader.read_line(&mut response_buf).await?;
        let response: ServerResponse = serde_json::from_str(&response_buf)?;

        Ok(response)
    }
}
