use tempfile::TempDir;
use tokio::{task, sync::mpsc};
use serde::{Deserialize, Serialize};

use std::sync::Arc;

use crate::nkv::{self, NotifyKeyValue};
use crate::request_msg;
use http::StatusCode;

pub struct PutMsg {
    key: String,
    value: Box<[u8]>,
    resp_tx: mpsc::Sender<nkv::NotifyKeyValueError>,
}

pub struct NkvGetResp {
    err: nkv::NotifyKeyValueError,
    value: Option<Arc<[u8]>>,
}
pub struct GetMsg {
    key: String,
    resp_tx: mpsc::Sender<NkvGetResp>,
}

pub struct BaseMsg {
    key: String,
    resp_tx: mpsc::Sender<nkv::NotifyKeyValueError>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
struct BaseResp {
    id: u32,

    #[serde(with = "http_serde::status_code")]
    status: StatusCode,
    message: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct GetResp {
    #[serde(flatten)]
    base: BaseResp,    
    data: Vec<u8>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
enum ServerResponse {
    Base(BaseResp),
    Get(GetResp),
}

pub struct Server {
    put_tx: mpsc::UnboundedSender<PutMsg>,
    get_tx: mpsc::UnboundedSender<GetMsg>,
    del_tx: mpsc::UnboundedSender<BaseMsg>,
    sub_tx: mpsc::UnboundedSender<BaseMsg>,
    unsub_tx: mpsc::UnboundedSender<BaseMsg>,
}

impl Server {
    pub fn new(nats_url: String, path: std::path::PathBuf) -> Self {

        let (put_tx, mut put_rx) = mpsc::unbounded_channel::<PutMsg>();
        let (get_tx, mut get_rx) = mpsc::unbounded_channel::<GetMsg>();
        let (del_tx, mut del_rx) = mpsc::unbounded_channel::<BaseMsg>();
        let (sub_tx, mut sub_rx) = mpsc::unbounded_channel::<BaseMsg>();
        let (unsub_tx, mut unsub_rx) = mpsc::unbounded_channel::<BaseMsg>();
        let (_cancel_tx, mut cancel_rx) = mpsc::unbounded_channel::<BaseMsg>();

        let mut nkv = NotifyKeyValue::new(path);

        tokio::spawn(async move {
            let mut cancelled = false;
            while !cancelled {
                tokio::select! {
                    Some(req) = put_rx.recv() => { 
                        nkv.put(&req.key, req.value);
                        let _ = req.resp_tx.send(nkv::NotifyKeyValueError::NoError).await;
                    }
                    Some(req) = get_rx.recv() => {
                        let _ = match nkv.get(&req.key) {
                            Some(resp) => req.resp_tx.send(NkvGetResp {
                                value: Some(resp),
                                err: nkv::NotifyKeyValueError::NoError
                            }).await,
                            None => req.resp_tx.send(NkvGetResp {
                                value: None,
                                err: nkv::NotifyKeyValueError::NotFound
                            }).await
                        };
                    }
                    Some(req) = del_rx.recv() => { 
                        nkv.delete(&req.key);
                        let _ = req.resp_tx.send(nkv::NotifyKeyValueError::NoError).await;
                    }
                    Some(req) = sub_rx.recv() => {
                        nkv.subscribe(req.key);
                        let _ = req.resp_tx.send(nkv::NotifyKeyValueError::NoError).await;
                    }
                    Some(req) = unsub_rx.recv() => {
                        nkv.unsubscribe(&req.key);
                        let _ = req.resp_tx.send(nkv::NotifyKeyValueError::NoError).await;
                    }
                    Some(_) = cancel_rx.recv() => { cancelled = true }
                    else => { break; }
                }
            }
        });

        let srv = Self { put_tx, get_tx, del_tx, sub_tx, unsub_tx };
        let get_tx_copy = srv.get_tx();
        let put_tx_copy = srv.put_tx();
        let del_tx_copy = srv.del_tx();
        let sub_tx_copy = srv.sub_tx();
        let unsub_tx_copy = srv.unsub_tx();

        tokio::spawn(async move {
            let nc = nats::asynk::connect(nats_url).await.unwrap();
            let sub = nc.subscribe("pubsub.*").await.unwrap();
            while let Some(msg) = sub.next().await {
                let get_tx = get_tx_copy.clone(); 
                let put_tx = put_tx_copy.clone();
                let del_tx = del_tx_copy.clone();
                let sub_tx = sub_tx_copy.clone();
                let unsub_tx = unsub_tx_copy.clone();

                task::spawn(async move {
                    let body = std::str::from_utf8(&msg.data).unwrap_or("");
                    let json_body: request_msg::MessageBody = serde_json::from_str(body).unwrap();
                    let reply = match msg.subject.as_str() {
                        "pubsub.get" => Self::handle_get(get_tx, json_body).await,
                        "pubsub.put" => Self::handle_put(put_tx, json_body).await, 
                        "pubsub.delete" => Self::handle_basic_msg(del_tx, json_body).await,
                        "pubsub.subscribe" => Self::handle_basic_msg(sub_tx, json_body).await, 
                        "pubsub.unsubscribe" => Self::handle_basic_msg(unsub_tx, json_body).await, 
                        _ => ServerResponse::Base(BaseResp{id: 0, status: StatusCode::OK, message: "tmp".to_string()}),
                    };
                    let json_reply = serde_json::to_string(&reply).unwrap();
                    msg.respond(json_reply).await.unwrap();
                });
            }
        });

        srv
    }

    async fn handle_put(nkv_tx: mpsc::UnboundedSender<PutMsg>, req: request_msg::MessageBody) -> ServerResponse {
        match req {
            request_msg::MessageBody::Put(request_msg::PutMessage {base, value}) => {
                let (resp_tx, mut resp_rx) = mpsc::channel(1);
                // TODO: handle error and throw response
                let _ = nkv_tx.send(PutMsg{key: base.key, value: value , resp_tx: resp_tx});
                let nkv_resp = resp_rx.recv().await.unwrap();
                let resp = BaseResp {
                    id: base.id,
                    status: nkv_resp.to_http_status(),
                    message: nkv_resp.to_string(),
                };
                ServerResponse::Base(resp) 
            }
            _ => { 
                let resp = BaseResp {
                    id: 0,
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    message: "wrong message for put handle".to_string(),
                };
                ServerResponse::Base(resp) 
            }
        }
    }

    async fn handle_get(nkv_tx: mpsc::UnboundedSender<GetMsg>, req: request_msg::MessageBody) -> ServerResponse {
        match req {
            request_msg::MessageBody::Get(request_msg::BaseMessage {id, key}) => {
                let (get_resp_tx, mut get_resp_rx) = mpsc::channel(1);
                let _ = nkv_tx.send(GetMsg{key: key, resp_tx: get_resp_tx});
                let nkv_resp = get_resp_rx.recv().await.unwrap();
                let mut data: Vec<u8> = Vec::new(); 
                if let Some(v) = nkv_resp.value {
                    data = v.to_vec();
                }
                let resp = GetResp {
                    base: BaseResp {
                        id: id,
                        status: nkv_resp.err.to_http_status(),
                        message: nkv_resp.err.to_string(),
                    },
                    data: data,
                };
                ServerResponse::Get(resp) 
            }
            _ => { 
                let resp = BaseResp {
                    id: 0,
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    message: "wrong message for get  handle".to_string(),
                };
                ServerResponse::Base(resp) 
            }
        }
    }

    async fn handle_basic_msg(nkv_tx: mpsc::UnboundedSender<BaseMsg>, req: request_msg::MessageBody) -> ServerResponse {
        match req {
            request_msg::MessageBody::Delete(request_msg::BaseMessage {id, key}) => {
                let (resp_tx, mut resp_rx) = mpsc::channel(1);
                let _ = nkv_tx.send(BaseMsg{key: key, resp_tx: resp_tx});
                let nkv_resp = resp_rx.recv().await.unwrap();
                let resp = BaseResp {
                    id: id,
                    status: nkv_resp.to_http_status(),
                    message: nkv_resp.to_string(),
                };
                ServerResponse::Base(resp) 
            }
            _ => { 
                let resp = BaseResp {
                    id: 0,
                    status: StatusCode::INTERNAL_SERVER_ERROR,
                    message: "wrong message for the handle".to_string(),
                };
                ServerResponse::Base(resp) 
            }
        }
    }

    pub fn put_tx(&self) -> mpsc::UnboundedSender<PutMsg> { self.put_tx.clone() }
    pub fn get_tx(&self) -> mpsc::UnboundedSender<GetMsg> { self.get_tx.clone() }
    pub fn del_tx(&self) -> mpsc::UnboundedSender<BaseMsg> { self.del_tx.clone() }
    pub fn sub_tx(&self) -> mpsc::UnboundedSender<BaseMsg> { self.sub_tx.clone() }
    pub fn unsub_tx(&self) -> mpsc::UnboundedSender<BaseMsg> { self.unsub_tx.clone() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;

    #[tokio::test]
    async fn test_server() { 
        let temp_dir = TempDir::new().expect("Failed to create temporary directory");
        let srv = Server::new("localhost:4442".to_string(), temp_dir.path().to_path_buf());

        let value: Box<[u8]> = Box::new([1, 2, 3, 4, 5]);
        let key = "key1".to_string();
        let (resp_tx, mut resp_rx) = mpsc::channel(1);

        let _ = srv.put_tx().send(PutMsg{key: key.clone(), value: value.clone(), resp_tx: resp_tx.clone()});

        let message = resp_rx.recv().await.unwrap();
        assert_eq!(message, nkv::NotifyKeyValueError::NoError);

        let (get_resp_tx, mut get_resp_rx) = mpsc::channel(1);
        let _ = srv.get_tx().send(GetMsg{key: key.clone(), resp_tx: get_resp_tx.clone()});
        let got = get_resp_rx.recv().await.unwrap();
        assert_eq!(got.err, nkv::NotifyKeyValueError::NoError);
        assert_eq!(got.value.unwrap(), value.into());

        let _ = srv.del_tx().send(BaseMsg{key: key.clone(), resp_tx: resp_tx.clone()});
        let got = resp_rx.recv().await.unwrap();
        assert_eq!(got, nkv::NotifyKeyValueError::NoError);

        let _ = srv.get_tx().send(GetMsg{key: key.clone(), resp_tx: get_resp_tx.clone()});
        let got = get_resp_rx.recv().await.unwrap();
        assert_eq!(got.err, nkv::NotifyKeyValueError::NotFound);
    }
}
