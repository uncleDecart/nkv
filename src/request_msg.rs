use serde::{Deserialize, Serialize};
use http::StatusCode;

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
}

#[derive(Debug, serde::Deserialize, serde::Serialize, PartialEq)]
pub struct BaseResp {
    pub id: u32,

    #[serde(with = "http_serde::status_code")]
    pub status: StatusCode,
    pub message: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct GetResp {
    #[serde(flatten)]
    pub base: BaseResp,    
    pub data: Vec<u8>,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct PutResp {
    #[serde(flatten)]
    pub base: BaseResp,    
    pub data: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq)]
#[serde(untagged)]
pub enum ServerResponse {
    Base(BaseResp),
    Get(GetResp),
    Put(PutResp),
}
