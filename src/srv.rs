use tempfile::TempDir;
use tokio::sync::mpsc;

use std::sync::Arc;

use crate::nkv::{self, NotifyKeyValue};


pub struct PutMsg {
    key: String,
    value: Box<[u8]>,
    resp_tx: mpsc::Sender<nkv::NotifyKeyValueError>,
}

pub struct GetMsg {
    key: String,
    resp_tx: mpsc::Sender<GetResp>,
}
pub struct GetResp {
    data: Option<Arc<[u8]>>,
    err: nkv::NotifyKeyValueError, 
}

pub struct BaseMsg {
    key: String,
    resp_tx: mpsc::Sender<nkv::NotifyKeyValueError>,
}

pub struct Server {
    put_tx: mpsc::UnboundedSender<PutMsg>,
    get_tx: mpsc::UnboundedSender<GetMsg>,
    del_tx: mpsc::UnboundedSender<BaseMsg>,
    sub_tx: mpsc::UnboundedSender<BaseMsg>,
    unsub_tx: mpsc::UnboundedSender<BaseMsg>,
}

impl Server {
    pub fn new(path: std::path::PathBuf) -> Self {
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
                            Some(resp) => req.resp_tx.send(GetResp {
                                data: Some(resp),
                                err: nkv::NotifyKeyValueError::NoError
                            }).await,
                            None => req.resp_tx.send(GetResp {
                                data: None,
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

        Self { put_tx, get_tx, del_tx, sub_tx, unsub_tx }
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
        let srv = Server::new(temp_dir.path().to_path_buf());

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
        assert_eq!(got.data.unwrap(), value.into());

        let _ = srv.del_tx().send(BaseMsg{key: key.clone(), resp_tx: resp_tx.clone()});
        let got = resp_rx.recv().await.unwrap();
        assert_eq!(got, nkv::NotifyKeyValueError::NoError);

        let _ = srv.get_tx().send(GetMsg{key: key.clone(), resp_tx: get_resp_tx.clone()});
        let got = get_resp_rx.recv().await.unwrap();
        assert_eq!(got.err, nkv::NotifyKeyValueError::NotFound);
    }
}
