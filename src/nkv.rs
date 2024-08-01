use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::persist_value::PersistValue;
use crate::notifier::Notifier;

struct Value {
    pv: PersistValue,
    notifier: Notifier,
}

pub struct NotifyKeyValue {
    state: Arc<RwLock<HashMap<String, Value>>>,
    persist_path:  PathBuf,
    sock_path: String,
}

impl NotifyKeyValue {
    pub fn new(path: std::path::PathBuf) -> Self {
        let temp_dir = TempDir::new().unwrap();

        Self{
            state: Arc::new(RwLock::new(HashMap::new())),
            persist_path: path,
            sock_path: "nats://localhost:4222".to_string(),
        }
    }

    pub async fn put(&self, key: &str, value: Box<[u8]>) {
        let mut map = self.state.write().await;

        if let Some(val) = map.get_mut(key) {
            val.pv.update(value);
            val.notifier.send_update(&*val.pv.data());
        } else {
            let path = self.persist_path.join(key);
            let val = PersistValue::new(value, path).expect("TOREMOVE");

            let notifier = Notifier::new(self.sock_path.clone()).expect("TOREMOVE");
            map.insert(key.to_string(), Value{
                pv: val,
                notifier: notifier,
            });
        }
    }

    pub async fn get(&self, key: &str) -> Option<Arc<[u8]>> {
        let map = self.state.read().await;
        map.get(key).map(|value| Arc::clone(&value.pv.data()))
    }

    pub async fn delete(&self, key: &str) {
        let mut map = self.state.write().await;
        map.remove(key);
    }

    pub async fn update_addr(&self, key: &str) -> Option<String> {
        let map = self.state.read().await;
        map.get(key).map(|value| value.notifier.addr())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio;
    use anyhow::Result;

    #[tokio::test]
    async fn test_put_and_get() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let nkv = NotifyKeyValue::new(temp_dir.path().to_path_buf());

        let data: Box<[u8]> = Box::new([1, 2, 3, 4, 5]);
        nkv.put("key1", data.clone()).await;

        let result = nkv.get("key1").await;
        assert_eq!(result, Some(Arc::from(data)));
        
        Ok(())
    }   

    #[tokio::test]
    async fn test_get_nonexistent_key() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let nkv = NotifyKeyValue::new(temp_dir.path().to_path_buf());

        let result = nkv.get("nonexistent_key").await;
        assert_eq!(result, None);

        Ok(())
    }

    #[tokio::test]
    async fn test_delete() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let nkv = NotifyKeyValue::new(temp_dir.path().to_path_buf());

        let data: Box<[u8]> = Box::new([1, 2, 3, 4, 5]);
        nkv.put("key1", data.clone()).await;

        nkv.delete("key1").await;
        let result = nkv.get("key1").await;
        assert_eq!(result, None);

        Ok(())
    }

    #[tokio::test]
    async fn test_update_value() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let nkv = NotifyKeyValue::new(temp_dir.path().to_path_buf());

        let data: Box<[u8]> = Box::new([1, 2, 3, 4, 5]);
        nkv.put("key1", data).await;

        let new_data: Box<[u8]> = Box::new([5, 6, 7, 8, 9]);
        nkv.put("key1", new_data.clone()).await;

        let result = nkv.get("key1").await;
        assert_eq!(result, Some(Arc::from(new_data)));

        Ok(())
    }

    #[tokio::test]
    async fn test_concurrent_writes() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let nkv = Arc::new(NotifyKeyValue::new(temp_dir.path().to_path_buf()));

        let key1 = "key1";
        let value1: Box<[u8]> = Box::new([1, 2, 3, 4]);
        let key2 = "key2";
        let value2: Box<[u8]> = Box::new([5, 6, 7, 8]);

        let kv_store_clone1 = Arc::clone(&nkv);
        let kv_store_clone2 = Arc::clone(&nkv);

        let v1 = value1.clone();
        let task1 = tokio::spawn(async move {
            kv_store_clone1.put(key1, v1).await;
        });

        let v2 = value2.clone();
        let task2 = tokio::spawn(async move {
            kv_store_clone2.put(key2, v2.clone()).await;
        });

        tokio::join!(task1, task2).0.unwrap();

        let result1 = nkv.get(key1).await;
        assert!(result1.is_some());
        assert_eq!(&*result1.unwrap(), value1.as_ref());

        let result2 = nkv.get(key2).await;
        assert!(result2.is_some());
        assert_eq!(&*result2.unwrap(), value2.as_ref());

        Ok(())
    }
}
