use std::collections::HashMap;
use std::path::PathBuf;
use tempfile::TempDir;

use std::sync::Arc;
use tokio::sync::RwLock;

use crate::persist_value::PersistValue;

pub struct NotifyKeyValue {
    state: Arc<RwLock<HashMap<String, PersistValue>>>,
    persist_path:  PathBuf,
}

impl NotifyKeyValue {
    pub fn new(path: std::path::PathBuf) -> Self {
        Self{
            state: Arc::new(RwLock::new(HashMap::new())),
            persist_path: path,
        }
    }

    pub async fn put(&self, key: &str, value: Box<[u8]>) {
        let mut map = self.state.write().await;

        let path = self.persist_path.join(key);
        let val = PersistValue::new(value, path);
        map.insert(key.to_string(), val.expect("TOREMOVE"));
    }

    pub async fn get(&self, key: &str) -> Option<Arc<[u8]>> {
        let map = self.state.read().await;
        map.get(key).map(|value| Arc::clone(&value.data()))
    }

    pub async fn delete(&self, key: &str) {
        let mut map = self.state.write().await;
        map.remove(key);
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
}
