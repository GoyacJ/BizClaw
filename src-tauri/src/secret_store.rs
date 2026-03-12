use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::{fs, io::ErrorKind};

use anyhow::{anyhow, Result};

pub trait SecretStore {
    fn set_secret(&self, value: &str) -> Result<()>;
    fn get_secret(&self) -> Result<Option<String>>;
    fn clear_secret(&self) -> Result<()>;
}

#[derive(Clone, Default)]
pub struct MemorySecretStore {
    token: Arc<Mutex<Option<String>>>,
}

impl SecretStore for MemorySecretStore {
    fn set_secret(&self, value: &str) -> Result<()> {
        let mut guard = self
            .token
            .lock()
            .map_err(|_| anyhow!("无法写入内存密钥仓库"))?;
        *guard = Some(value.to_string());
        Ok(())
    }

    fn get_secret(&self) -> Result<Option<String>> {
        let guard = self
            .token
            .lock()
            .map_err(|_| anyhow!("无法读取内存密钥仓库"))?;
        Ok(guard.clone())
    }

    fn clear_secret(&self) -> Result<()> {
        let mut guard = self
            .token
            .lock()
            .map_err(|_| anyhow!("无法清理内存密钥仓库"))?;
        *guard = None;
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct LocalSecretStore {
    path: PathBuf,
}

impl LocalSecretStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl SecretStore for LocalSecretStore {
    fn set_secret(&self, value: &str) -> Result<()> {
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)?;
        } else {
            return Err(anyhow!("Token 存储路径缺少父目录"));
        }

        fs::write(&self.path, value)?;
        Ok(())
    }

    fn get_secret(&self) -> Result<Option<String>> {
        match fs::read_to_string(&self.path) {
            Ok(content) => {
                let token = content.trim_end_matches(['\r', '\n']).to_string();
                if token.is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(token))
                }
            }
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn clear_secret(&self) -> Result<()> {
        match fs::remove_file(&self.path) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use tempfile::tempdir;

    use super::{LocalSecretStore, MemorySecretStore, SecretStore};

    #[test]
    fn memory_secret_store_round_trip() {
        let store = MemorySecretStore::default();

        store.set_secret("gateway-token").unwrap();
        let token = store.get_secret().unwrap();

        assert_eq!(token.as_deref(), Some("gateway-token"));

        store.clear_secret().unwrap();
        assert_eq!(store.get_secret().unwrap(), None);
    }

    #[test]
    fn local_secret_store_round_trip() {
        let temp = tempdir().unwrap();
        let store = LocalSecretStore::new(temp.path().join("gateway-token.txt"));

        store.set_secret("gateway-token").unwrap();
        assert_eq!(store.get_secret().unwrap().as_deref(), Some("gateway-token"));

        store.clear_secret().unwrap();
        assert_eq!(store.get_secret().unwrap(), None);
    }

    #[test]
    fn blank_local_secret_file_is_treated_as_missing() {
        let temp = tempdir().unwrap();
        let path = temp.path().join("gateway-token.txt");
        std::fs::write(&path, "\n").unwrap();
        let store = LocalSecretStore::new(path);

        assert_eq!(store.get_secret().unwrap(), None);
    }
}
