use std::fs;
use std::path::{Path, PathBuf};
use tempfile::{TempDir, NamedTempFile};
use anyhow::{Result, Context};
use std::io::Write;

#[derive(Debug)]
pub struct PersistValue {
    data: Box<[u8]>,
    fp: PathBuf,
}

impl PersistValue {
    pub fn new(new_data: Box<[u8]>, filepath: PathBuf) -> Result<Self> {
        atomic_write(&*new_data, &filepath)?;

        Ok(Self {
            data: new_data,
            fp: filepath,
        })
    }

    pub fn from_checkpoint(filepath: &Path) -> Result<Self> {
        let data = fs::read(filepath)
            .with_context(|| format!("Failed to read file: {:?}", filepath))?;
        
        Ok(Self {
            data: data.into_boxed_slice(),
            fp: filepath.to_path_buf(),
        })
    }
    
    pub fn update(&mut self, new_data: Box<[u8]>) -> Result<()> {
        atomic_write(&*new_data, &self.fp)?;

        self.data = new_data;
        Ok(())
    }

    pub fn delete_checkpoint(&self) -> Result<()> {
        fs::remove_file(&self.fp)
            .with_context(|| format!("Failed to remove file: {:?}", self.fp))?;
        Ok(())
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

impl PartialEq for PersistValue {
    fn eq(&self, other: &Self) -> bool {
        self.fp == other.fp && self.data == other.data
    }
}

fn atomic_write(data: &[u8], filename: &Path) -> Result<()> {
    let mut tmp_file = NamedTempFile::new()?;
    tmp_file.write_all(data)?;

    let tmp_path = tmp_file.path();
    fs::rename(tmp_path, filename)
        .with_context(|| format!("Failed to rename temporary file to: {:?}", filename))?;
    Ok(())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persist_value() -> Result<()> {
       let temp_dir = TempDir::new()?;
       let file_path = temp_dir.path().join("testfile.dat");

       let original_data: Box<[u8]> = Box::new([1, 2, 3, 4, 5]);

       let persist_value = PersistValue::new(original_data.clone(), file_path.clone())?;

       let data_from_file = fs::read(file_path.clone())?;
       assert_eq!(data_from_file, original_data.as_ref());

       let loaded_persist_value = PersistValue::from_checkpoint(file_path.as_path())?;

       assert_eq!(persist_value, loaded_persist_value);

       temp_dir.close()?;

       Ok(())
    }   

    #[test]
    fn test_update() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("testfile.dat");

        let initial_data: Box<[u8]> = Box::new([1, 2, 3, 4, 5]);

        let mut persist_value = PersistValue::new(initial_data.clone(), file_path.clone())?;

        let new_data: Box<[u8]> = Box::new([6, 7, 8, 9, 10]);

        persist_value.update(new_data.clone())?;

        let data_from_file = fs::read(file_path.clone())?;
        assert_eq!(data_from_file, new_data.as_ref());

        assert_eq!(persist_value.data(), new_data.as_ref());

        temp_dir.close()?;

        Ok(())
    }

    #[test]
    fn test_delete_checkpoint() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let file_path = temp_dir.path().join("testfile.dat");

        let initial_data: Box<[u8]> = Box::new([1, 2, 3, 4, 5]);

        let persist_value = PersistValue::new(initial_data.clone(), file_path.clone())?;

        assert!(file_path.exists());

        persist_value.delete_checkpoint()?;

        assert!(!file_path.exists());

        temp_dir.close()?;

        Ok(())
    }

}
