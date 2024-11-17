use std::{fs::create_dir_all, path::{Path, PathBuf}};

use app_dirs2::{AppDataType, AppInfo, app_root};
use anyhow::{Ok, Result, Context};
use keyring::Entry;
use rusqlite::Connection;

pub struct Credential {
    user: String,
    password: String,
    totp_command: Option<String>
}

impl Credential {
    pub fn new(user: &str, password: &str, totp_command: Option<&str>) -> Credential {
        let totp_command = match totp_command {
            Some(totp_command) => Some(totp_command.to_string()),
            None => None
        };

        Credential {
            user: user.to_string(),
            password: password.to_string(),
            totp_command
        }
    }
}

pub struct CredentialManager {
}

impl CredentialManager {
    fn get_database(&self, sqlite_path: &Path) -> Result<Connection> {
        // Get the path to the credential database
        let mut path = app_root(AppDataType::UserConfig, &AppInfo{
            name: "git-lfs-synology",
            author: "Engineers for Exploration"
        })?;
        path.push("credential_store.db");

        // Create the folder if it doesn't already exist.
        if !sqlite_path.parent().context("No parent")?.exists(){
            create_dir_all(sqlite_path.parent().context("No parent")?)?;
        }

        let should_init_database = !sqlite_path.exists();
        let conn = Connection::open(sqlite_path)?;

        if should_init_database {
            // TODO Create tables
        }

        Ok(conn)
    }

    pub fn get_credential(&self, url: &str) -> Result<Credential> {
        let user = "";
        let password = "";

        // match self.get_password(url) {
        //     Ok(password) => {
        //         let user = "";
        //         let totp_command = Some("");

        //         Some(Credential::new(user, password.as_str(), None))
        //     },
        //     Err(error) => None
        // }

        Ok(Credential::new(user, password, None))
    }

    pub fn has_credential(&self, url: &str) -> bool {        
        false
    }

    pub fn remove_credential(&self, url: &str) {
        //self.delete_entry_keyring(url)
    }

    pub fn set_credential(&self, url: &str, credential: &Credential) {
        

        // todo insert totp command
    }
}