use anyhow::{anyhow, Result};
use keyring::Entry;
use std::sync::{Arc, Mutex};

pub trait SecretStore {
    fn set_token(&self, token: &str) -> Result<()>;
    fn get_token(&self) -> Result<Option<String>>;
    fn clear_token(&self) -> Result<()>;
}

#[derive(Clone, Default)]
pub struct MemorySecretStore {
    token: Arc<Mutex<Option<String>>>,
}

impl SecretStore for MemorySecretStore {
    fn set_token(&self, token: &str) -> Result<()> {
        let mut guard = self
            .token
            .lock()
            .map_err(|_| anyhow!("无法写入内存密钥仓库"))?;
        *guard = Some(token.to_string());
        Ok(())
    }

    fn get_token(&self) -> Result<Option<String>> {
        let guard = self
            .token
            .lock()
            .map_err(|_| anyhow!("无法读取内存密钥仓库"))?;
        Ok(guard.clone())
    }

    fn clear_token(&self) -> Result<()> {
        let mut guard = self
            .token
            .lock()
            .map_err(|_| anyhow!("无法清理内存密钥仓库"))?;
        *guard = None;
        Ok(())
    }
}

const SERVICE_NAME: &str = "com.goyacj.bizclaw";
const TOKEN_ACCOUNT: &str = "OPENCLAW_GATEWAY_TOKEN";

pub struct KeyringSecretStore {
    entry: Entry,
}

impl KeyringSecretStore {
    pub fn new() -> Result<Self> {
        Ok(Self {
            entry: Entry::new(SERVICE_NAME, TOKEN_ACCOUNT)?,
        })
    }
}

impl SecretStore for KeyringSecretStore {
    fn set_token(&self, token: &str) -> Result<()> {
        self.entry.set_password(token)?;
        Ok(())
    }

    fn get_token(&self) -> Result<Option<String>> {
        match self.entry.get_password() {
            Ok(token) => Ok(Some(token)),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(error) => Err(error.into()),
        }
    }

    fn clear_token(&self) -> Result<()> {
        match self.entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(error) => Err(error.into()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{MemorySecretStore, SecretStore};

    #[test]
    fn memory_secret_store_round_trip() {
        let store = MemorySecretStore::default();

        store.set_token("gateway-token").unwrap();
        let token = store.get_token().unwrap();

        assert_eq!(token.as_deref(), Some("gateway-token"));

        store.clear_token().unwrap();
        assert_eq!(store.get_token().unwrap(), None);
    }
}
