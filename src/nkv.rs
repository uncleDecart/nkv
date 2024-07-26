mod persist_value;

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use std::io::{self, Write};


pub struct NotifyKeyValue {
    state HashMap<str, PersistValue>;
    persist_path PathBuf;
}

impl NotifyKeyValue {
    pub fn new(path std::path::Path) -> Result<Self> {}

    pub async fn put(key String, value [u8]) -> Result<()> {}
    pub async fn get(key String) -> Result<&[u8]> {}
    pub async fn delete(key String) -> Result<()> {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        assert_eq!(true);
    }

    #[test]
    fn test_put() {
        tmp_dir;
        let nkv = new(tmp_dir)
    }   
}
