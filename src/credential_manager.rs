use std::{collections::HashMap, fs::create_dir_all, process::Command};

use aes_gcm::{aead::{Aead, OsRng}, AeadCore, Aes256Gcm, Key, KeyInit, Nonce};
use app_dirs2::{AppDataType, AppInfo, app_root};
use anyhow::{Result, Context};
use keyring::{credential, Entry};
use rusqlite::Connection;

#[derive(Debug)]
struct DatabaseCredential {
    url: String,
    user: String,
    totp_comand_encrypted: Option<Vec<u8>>,
    totp_nonce: Option<Vec<u8>>
}

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

        Credential::new_with_string(user.to_string(), password.to_string(), totp_command)
    }

    pub fn new_with_string(user: String, password: String, totp_command: Option<String>) -> Credential {
        Credential {
            user,
            password,
            totp_command
        }
    }

    pub fn totp(&self) -> Option<String> {
        match self.totp_command.clone() {
            Some(totp_command) => {
                let parts = totp_command.split(" ").collect::<Vec<&str>>();
                let command = parts.first()?.to_string();
                let args = totp_command[..command.len()].to_string();

                let output = Command::new(command)
                     .arg(args)
                     .output()
                     .expect("failed to execute process");

                Some(String::from_utf8_lossy(&output.stdout).to_string())
            },
            None => None
        }
    }
}

pub struct CredentialManager {
    connection: Connection,
    entry_cache: HashMap<(String, String), Entry>
}

impl CredentialManager {
    pub fn new() -> Result<CredentialManager> {
        Ok(CredentialManager {
            connection: CredentialManager::get_connection()?,
            entry_cache: HashMap::new()
        })
    }

    fn get_connection() -> Result<Connection> {
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

        Ok(Connection::open(sqlite_path)?)
    }

    fn get_database_credential_iter(&self, url: &str) -> Result<Vec<DatabaseCredential>> {
        let database = self.get_database()?;

        let mut stmt: rusqlite::Statement<'_> = database.prepare(
            "SELECT url, user, totp_command_encrypted, totp_nonce FROM Credentials WHERE url=:url;")?;
        let rows: Vec<DatabaseCredential> = stmt.query_map(&[(":url", url)], |row| {
            Ok(DatabaseCredential {
                url: row.get(0)?,
                user: row.get(1)?,
                totp_comand_encrypted: row.get(2)?,
                totp_nonce: row.get(3)?
            })
        })?.filter_map(|r| r.ok()).collect::<Vec<DatabaseCredential>>().try_into()?;
        
        Ok(rows)
    }

    fn get_database(&self) -> Result<&Connection> {
        let conn = &self.connection;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS Credentials (
                id                      INTEGER PRIMARY KEY,
                url                     TEXT NOT NULL,
                user                    TEXT NOT NULL,
                totp_command_encrypted  BLOB,
                totp_nonce              BLOB
            )",
            (), // empty list of parameters.
        )?;

        Ok(conn)
    }

    fn get_entry(&mut self, url: &str, user: &str) -> Result<&Entry>{
        if !self.entry_cache.contains_key(&(url.to_string(), user.to_string())) {
            let entry = Entry::new(url, user)?;
            self.entry_cache.insert((url.to_string(), user.to_string()), entry);
        }

        Ok(self.entry_cache.get(&(url.to_string(), user.to_string())).context("Entry does not exist in cache")?)
    }

    fn pad_string(&self, input: &str) -> String {
        let mut output = input.to_string();

        while output.len() < 32 {
            output.push(' ');
        }

        output
    }

    pub fn get_credential(&mut self, url: &str) -> Result<Option<Credential>> {
        if !self.has_credential(url)? {
            return Ok(None);
        }

        let database_rows = self.get_database_credential_iter(url)?;
        let database_credential = database_rows.first().context("No elements returned from database.")?;
        let entry = self.get_entry(url, &database_credential.user)?;

        let password = entry.get_password()?;

        let mut totp_command: Option<String> = None;
        if database_credential.totp_comand_encrypted.is_some() {
            let padded_password = self.pad_string(password.as_str());
            let key: &Key<Aes256Gcm> = padded_password.as_bytes().into();

            let nonce_vec = database_credential.totp_nonce.clone().context("No nonce provided for credential")?;

            let cipher = Aes256Gcm::new(&key);
            let nonce = Nonce::from_iter(nonce_vec);
            let plaintext = cipher.decrypt(
                &nonce, 
                database_credential.totp_comand_encrypted.clone().context("TOTP command is empty.")?.as_ref())?;
            totp_command = Some(String::from_utf8(plaintext)?);
        }

        Ok(Some(Credential::new_with_string(database_credential.user.clone(), password, totp_command)))
    }

    pub fn has_credential(&self, url: &str) -> Result<bool> {
        let database_rows = self.get_database_credential_iter(url)?;

        Ok(!database_rows.is_empty())
    }

    pub fn remove_credential(&mut self, url: &str) -> Result<()> {
        if self.has_credential(url)? {
            let database_rows = self.get_database_credential_iter(url)?;
            let database_credential = database_rows.first().context("No elements returned from database.")?;

            let entry = self.get_entry(url, &database_credential.user)?;
            entry.delete_credential()?;
            self.entry_cache.remove(&(url.to_string(), database_credential.user.to_string()));

            let database = self.get_database()?;

            database.execute(
                "DELETE FROM Credentials WHERE url=?1",
                [url].map(|n| n.to_string()),
            )?;
        }

        Ok(())
    }

    pub fn set_credential(&mut self, url: &str, credential: &Credential) -> Result<()> {
        if self.has_credential(url)? {
            self.remove_credential(url)?;
        }

        let mut totp_comand_encrypted: Option<Vec<u8>> = None;
        let mut totp_nonce: Option<Vec<u8>> = None;
        if credential.totp_command.is_some() {
            let padded_password = self.pad_string(credential.password.as_str());
            let key: &Key<Aes256Gcm> = padded_password.as_bytes().into();

            let cipher = Aes256Gcm::new(&key);
            let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits; unique per message
            let ciphertext = cipher.encrypt(&nonce, credential.totp_command.clone().context("TOTP Command does not exist.")?.as_bytes())?;

            totp_comand_encrypted = Some(ciphertext);
            totp_nonce = Some(nonce.as_slice().to_vec());
        }

        let database = self.get_database()?;
        database.execute(
            "INSERT INTO Credentials (url, user, totp_command_encrypted, totp_nonce) VALUES (?1, ?2, ?3, ?4)",
            (
                url.to_string(),
                credential.user.to_string(),
                totp_comand_encrypted,
                totp_nonce,
        ))?;

        let entry = self.get_entry(url, &credential.user)?;
        entry.set_password(&credential.password)?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use anyhow::Context;
    use keyring::{mock, set_default_credential_builder};
    use rusqlite::Connection;

    use super::{CredentialManager, Credential};

    #[test]
    fn create_credential_manager() {
        let _ = CredentialManager::new();
    }

    #[test]
    fn set_get_credential_no_totp_command() {
        set_default_credential_builder(mock::default_credential_builder()); // Set mock

        let mut credential_manager = CredentialManager {
            connection: Connection::open_in_memory().unwrap(),
            entry_cache: HashMap::new()
        };

        credential_manager.set_credential("http://example.com", &Credential::new("test_user", "test_password", None)).unwrap();

        let credential: Credential = credential_manager.get_credential("http://example.com").unwrap().context("Credential expected").unwrap();

        assert_eq!(credential.user, "test_user".to_string());
        assert_eq!(credential.password, "test_password".to_string());
        assert_eq!(credential.totp_command, None);
    }

    #[test]
    fn set_get_credential_totp_command() {
        set_default_credential_builder(mock::default_credential_builder()); // Set mock

        let mut credential_manager = CredentialManager {
            connection: Connection::open_in_memory().unwrap(),
            entry_cache: HashMap::new()
        };

        credential_manager.set_credential("http://example.com", &Credential::new("test_user", "test_password", Some("echo 12345"))).unwrap();

        let credential: Credential = credential_manager.get_credential("http://example.com").unwrap().context("Credential expected").unwrap();

        assert_eq!(credential.user, "test_user".to_string());
        assert_eq!(credential.password, "test_password".to_string());
        assert_eq!(credential.totp_command.context("Should not be null").unwrap(), "echo 12345".to_string());
    }

    #[test]
    fn has_credential() {
        set_default_credential_builder(mock::default_credential_builder()); // Set mock

        let mut credential_manager = CredentialManager {
            connection: Connection::open_in_memory().unwrap(),
            entry_cache: HashMap::new()
        };

        credential_manager.set_credential("http://example.com", &Credential::new("test_user", "test_password", Some("echo 12345"))).unwrap();

        assert!(credential_manager.has_credential("http://example.com").unwrap());
    }

    #[test]
    fn can_update_credential_no_totp() {
        set_default_credential_builder(mock::default_credential_builder()); // Set mock

        let mut credential_manager = CredentialManager {
            connection: Connection::open_in_memory().unwrap(),
            entry_cache: HashMap::new()
        };

        credential_manager.set_credential("http://example.com", &Credential::new("test_user", "test_password", Some("echo 12345"))).unwrap();

        credential_manager.set_credential("http://example.com", &Credential::new("test_user", "test_password2", None)).unwrap();

        let credential: Credential = credential_manager.get_credential("http://example.com").unwrap().context("Credential expected").unwrap();

        assert_eq!(credential.user, "test_user".to_string());
        assert_eq!(credential.password, "test_password2".to_string());
        assert_eq!(credential.totp_command, None);
    }

    #[test]
    fn can_update_credential_totp() {
        set_default_credential_builder(mock::default_credential_builder()); // Set mock

        let mut credential_manager = CredentialManager {
            connection: Connection::open_in_memory().unwrap(),
            entry_cache: HashMap::new()
        };

        credential_manager.set_credential("http://example.com", &Credential::new("test_user", "test_password", Some("echo 12345"))).unwrap();

        credential_manager.set_credential("http://example.com", &Credential::new("test_user", "test_password2", Some("echo 123456"))).unwrap();

        let credential: Credential = credential_manager.get_credential("http://example.com").unwrap().context("Credential expected").unwrap();

        assert_eq!(credential.user, "test_user".to_string());
        assert_eq!(credential.password, "test_password2".to_string());
        assert_eq!(credential.totp_command.context("TOTP should of not null").unwrap(), "echo 123456".to_string());
    }

    #[test]
    fn remove_credential() {
        set_default_credential_builder(mock::default_credential_builder()); // Set mock

        let mut credential_manager = CredentialManager {
            connection: Connection::open_in_memory().unwrap(),
            entry_cache: HashMap::new()
        };

        credential_manager.set_credential("http://example.com", &Credential::new("test_user", "test_password", Some("echo 12345"))).unwrap();
        credential_manager.remove_credential("http://example.com").unwrap();

        assert!(!credential_manager.has_credential("http://example.com").unwrap());
    }
}