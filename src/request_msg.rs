use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct BaseMessage {
    pub id: u32,
    pub key: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PutMessage {
    #[serde(flatten)]
    pub base: BaseMessage,
    pub value: Box<[u8]>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type")]
pub enum MessageBody {
    Put(PutMessage),
    Get(BaseMessage),
    Delete(BaseMessage),
    Subscribe(BaseMessage),
    Unsubscribe(BaseMessage),
}
