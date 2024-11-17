use std::fs::create_dir_all;

use aes_gcm::{aead::{Aead, OsRng}, AeadCore, Aes256Gcm, Key, KeyInit};
use app_dirs2::{AppDataType, AppInfo, app_root};
use anyhow::{Result, Context};
use keyring::Entry;
use rusqlite::Connection;

#[derive(Debug)]
struct DatabaseCredential {
    id: i32,
    url: String,
    user: String,
    totp_comand_encrypted: Option<Vec<u8>>,
}

pub struct Credential {
    user: String,
    password: String,
    totp_command: Option<String>
}

impl Credential {
    pub fn new(user: &str, password: &str, totp_command: Option<String>) -> Credential {
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
    fn get_database(&self) -> Result<Connection> {
        // Get the path to the credential database
        let mut path = app_root(AppDataType::UserConfig, &AppInfo{
            name: "git-lfs-synology",
            author: "Engineers for Exploration"
        })?;
        path.push("credential_store.db");
        let sqlite_path = path.as_path();

        // Create the folder if it doesn't already exist.
        if !sqlite_path.parent().context("No parent")?.exists(){
            create_dir_all(sqlite_path.parent().context("No parent")?)?;
        }

        let should_init_database = !sqlite_path.exists();
        let conn = Connection::open(sqlite_path)?;

        if should_init_database {
            conn.execute(
                "CREATE TABLE Credentials (
                    id                      INTEGER PRIMARY KEY,
                    url                     TEXT NOT NULL,
                    user                TEXT NOT NULL,
                    totp_command_encrypted  BLOB
                )",
                (), // empty list of parameters.
            )?;
        }

        Ok(conn)
    }

    pub fn get_credential(&self, url: &str) -> Result<Credential> {
        let database = self.get_database()?;

        let mut stmt = database.prepare(
            "SELECT id, url, user, totp_comand_encrypted FROM Credentials WHERE url = ?1")?;
        let database_credential_iter = stmt.query_map([url], |row| {
            Ok(DatabaseCredential {
                id: row.get(0)?,
                url: row.get(1)?,
                user: row.get(2)?,
                totp_comand_encrypted: row.get(3)?
            })
        })?;

        let database_credential = database_credential_iter.last().context("Database does not contain credential.")??;
        let entry = Entry::new(url, &database_credential.user)?;

        let password = entry.get_password()?;
        let mut totp_command: Option<String> = None;

        if database_credential.totp_comand_encrypted.is_some() {
            let key: &Key<Aes256Gcm> = password.as_bytes().into();

            let cipher = Aes256Gcm::new(&key);
            let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits; unique per message
            let plaintext = cipher.decrypt(
                &nonce, 
                database_credential.totp_comand_encrypted.context("TOTP command is empty.")?.as_ref())?;
            totp_command = Some(String::from_utf8(plaintext)?);
        }

        Ok(Credential::new(&database_credential.user, &password, totp_command))
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