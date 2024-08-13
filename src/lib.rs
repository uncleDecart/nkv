pub mod nkv;
mod notifier;
pub mod request_msg;
mod persist_value;

use crate::request_msg::*;

pub struct NatsClient {
    client: async_nats::Client,
}

impl NatsClient {
    pub async fn new(server: &str) -> Result<Self, async_nats::Error> {
        let client = async_nats::connect(server).await?;
        Ok(Self { client })
    }

    pub async fn get(&self, key: String) -> Result<ServerResponse, async_nats::Error> {
        let req = MessageBody::Get(BaseMessage{
            id: 0,
            key: key,
        });
        self.send_request(&req).await
    }

    pub async fn put(&self, key: String, val: Box<[u8]>) -> Result<ServerResponse, async_nats::Error> {
        let req = MessageBody::Put(PutMessage{
            base: BaseMessage{
                id: 0,
                key: key,
            },
            value: val,
        });
        self.send_request(&req).await
    }

    pub async fn delete(&self, key: String) -> Result<ServerResponse, async_nats::Error> {
        let req = MessageBody::Delete(BaseMessage{
            id: 0,
            key: key,
        });
        self.send_request(&req).await
    }

    pub async fn subscribe(&self, key: String) -> Result<ServerResponse, async_nats::Error> {
        let req = MessageBody::Subscribe(BaseMessage{
            id: 0,
            key: key,
        });
        self.send_request(&req).await
    }
    
    async fn send_request(&self, request: &MessageBody) -> Result<ServerResponse, async_nats::Error> {
        let req = serde_json::to_string(&request)?;
        let subj = match request {
            MessageBody::Put( .. ) => "pubsub.put",
            MessageBody::Get( .. ) => "pubsub.get",
            MessageBody::Delete( .. ) => "pubsub.delete",
            MessageBody::Subscribe( .. ) => "pubsub.subscribe",
        };
        let reply = self.client
            .request(subj, req.into())
            .await?;

        
        let response_data: ServerResponse = serde_json::from_slice(&reply.payload)?;
        Ok(response_data)
    }
}
