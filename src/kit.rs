use std::path::Path;
use std::sync::atomic::Ordering;
use std::sync::{atomic::AtomicBool, Arc, RwLock};

use anyhow::Ok;
use directories::ProjectDirs;

use crate::models::{CommandResponse, Device, DeviceListResult, MikitError};
use crate::network::CommandReqeust;
use crate::{models::MiAccount, network::HttpClient, store::DataSore};

pub struct MiKit {
    http_client: Arc<HttpClient>,
    db: Arc<DataSore>,
    account: Arc<RwLock<Option<MiAccount>>>,
    is_logged: AtomicBool,
}

impl MiKit {
    pub fn new(application_name: &str, organization_name: &str) -> anyhow::Result<Self> {
        let db = DataSore::new(application_name, organization_name)?;
        let account = db.get::<MiAccount>("account").ok();
        let is_logged = AtomicBool::new(account.is_some());
        Ok(MiKit {
            http_client: Arc::new(HttpClient::default()),
            db: Arc::new(db),
            account: Arc::new(RwLock::new(account)),
            is_logged,
        })
    }

    pub async fn login(&self, username: &str, password: &str) -> anyhow::Result<()> {
        let client = self.http_client.clone();
        let account = client.login(username, password).await?;

        let db = self.db.clone();
        db.set("account", &account)?;

        let mut guard = self.account.write().unwrap();
        *guard = Some(account.clone());

        self.is_logged.store(true, Ordering::Relaxed);

        Ok(())
    }

    pub async fn fetch_devices(&self) -> anyhow::Result<Vec<Device>> {
        if !self.is_logged() {
            return Err(MikitError::UnLogin.into());
        }
        let client = self.http_client.clone();
        let account = self.get_account().unwrap();
        Ok(client
            .execute_command::<CommandResponse<DeviceListResult>>(
                CommandReqeust::DeviceList,
                &account,
            )
            .await?
            .result
            .ok_or(MikitError::Unknown("parse data error".to_string()))?
            .list)
    }

    pub fn logout(&mut self) -> anyhow::Result<()> {
        let mut account = self.account.write().unwrap();
        *account = None;
        self.db.clear()
    }

    pub fn get_account(&self) -> Option<MiAccount> {
        let account = self.account.clone();
        let account = account.read().unwrap();
        account.as_ref().and_then(|value| Some(value.clone()))
    }

    pub fn is_logged(&self) -> bool {
        self.is_logged.load(Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::MiKit;

    #[tokio::test]
    async fn feature() {
        let mikit = MiKit::new("mikit", "com.nickming").unwrap();
        let account = mikit.get_account().unwrap();
        println!("{:?}", &account);
    }
}
