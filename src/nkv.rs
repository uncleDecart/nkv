use std::collections::HashMap;
use std::path::PathBuf;

use std::sync::Arc;

use crate::persist_value::PersistValue;
use crate::notifier::Notifier;
use std::fmt;
use std::env;

#[derive(Debug, PartialEq)]
pub enum NotifyKeyValueError {
    NoError,
    NotFound,
}

impl NotifyKeyValueError {
    pub fn to_http_status(&self) -> http::StatusCode {
        match self {
            NotifyKeyValueError::NotFound => http::StatusCode::NOT_FOUND,
            NotifyKeyValueError::NoError => http::StatusCode::OK,
        }
    }
}

impl fmt::Display for NotifyKeyValueError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NotifyKeyValueError::NotFound => write!(f, "Not Found"),
            NotifyKeyValueError::NoError => write!(f, "No Error"),
        }
    }
}

impl std::error::Error for NotifyKeyValueError {}

struct Value {
    pv: PersistValue,
    notifier: Notifier,
}

pub struct NotifyKeyValue {
    state: HashMap<String, Value>,
    persist_path:  PathBuf,
    sock_path: String,
}


impl NotifyKeyValue {
    pub fn new(path: std::path::PathBuf) -> Self {
        let nats_url = env::var("NATS_URL")
                    .unwrap_or_else(|_| "nats://localhost:4222".to_string());
        Self{
            state: HashMap::new(),
            persist_path: path,
            sock_path: nats_url,
        }
    }

    pub async fn put(&mut self, key: &str, value: Box<[u8]>) {
        if let Some(val) = self.state.get_mut(key) {
            let _ = val.pv.update(value);
            let _ = val.notifier.send_update(&*val.pv.data());
        } else {
            let path = self.persist_path.join(key);
            let val = PersistValue::new(value, path).expect("TOREMOVE");

            let notifier = Notifier::new(self.sock_path.clone(), format!("pub-{}", key).into()).await.expect("TOREMOVE");
            self.state.insert(key.to_string(), Value{
                pv: val,
                notifier: notifier,
            });
        }
    }

    pub fn get(&self, key: &str) -> Option<Arc<[u8]>> {
        self.state.get(key).map(|value| Arc::clone(&value.pv.data()))
    }

    pub fn delete(&mut self, key: &str) {
        self.state.remove(key);
    }

    pub fn subscribe(&mut self, key: &str) -> Option<String> {
        self.state.get(key).map(|value| value.notifier.topic())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;
    use anyhow::Result;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_put_and_get() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let mut nkv = NotifyKeyValue::new(temp_dir.path().to_path_buf());

        let data: Box<[u8]> = Box::new([1, 2, 3, 4, 5]);
        nkv.put("key1", data.clone()).await;

        let result = nkv.get("key1");
        assert_eq!(result, Some(Arc::from(data)));
        
        Ok(())
    }   

    #[tokio::test]
    async fn test_get_nonexistent_key() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let nkv = NotifyKeyValue::new(temp_dir.path().to_path_buf());

        let result = nkv.get("nonexistent_key");
        assert_eq!(result, None);

        Ok(())
    }

    #[tokio::test]
    async fn test_delete() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let mut nkv = NotifyKeyValue::new(temp_dir.path().to_path_buf());

        let data: Box<[u8]> = Box::new([1, 2, 3, 4, 5]);
        nkv.put("key1", data.clone()).await;

        nkv.delete("key1");
        let result = nkv.get("key1");
        assert_eq!(result, None);

        Ok(())
    }

    #[tokio::test]
    async fn test_update_value() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let mut nkv = NotifyKeyValue::new(temp_dir.path().to_path_buf());

        let data: Box<[u8]> = Box::new([1, 2, 3, 4, 5]);
        nkv.put("key1", data).await;

        let new_data: Box<[u8]> = Box::new([5, 6, 7, 8, 9]);
        nkv.put("key1", new_data.clone()).await;

        let result = nkv.get("key1");
        assert_eq!(result, Some(Arc::from(new_data)));

        Ok(())
    }
}
