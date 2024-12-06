use std::{collections::HashMap, fs::create_dir_all};

use aes_gcm::{aead::{Aead, OsRng}, AeadCore, Aes256Gcm, Key, KeyInit, Nonce};
use anyhow::{anyhow, bail, Context, Result};
use educe::Educe;
use keyring::Entry;
use rusqlite::Connection;
use thiserror::Error;
use tracing::{debug, info};

use crate::users_dirs::get_config_dir;

#[derive(Error, Debug)]
enum CredentialError {
    #[error("Sqlite database is not initialized.")]
    DatabaseNotInitialized,
    #[error(transparent)]
    SqliteError(#[from] rusqlite::Error),
    #[error(transparent)]
    AnyhowError(#[from] anyhow::Error)
}

#[derive(Debug)]
struct DatabaseCredential {
    user: String,
    device_id_encrypted: Option<Vec<u8>>,
    device_id_nonce: Option<Vec<u8>>
}

#[derive(Educe)]
#[educe(Debug)]
pub struct Credential {
    pub user: String,
    #[educe(Debug(ignore))] // Do not include password in logs.
    pub password: String,
    pub totp: Option<String>,
    pub device_id: Option<String>
}

impl Credential {
    #[tracing::instrument]
    pub fn new(user: String, password: String) -> Credential {
        Credential::new_totp(user, password, None)
    }

    #[tracing::instrument]
    pub fn new_totp(user: String, password: String, totp: Option<String>) -> Credential {
        Credential {
            user,
            password,
            totp,
            device_id: None
        }
    }
}

#[derive(Debug)]
pub struct CredentialManager {
    connection: Connection,
    entry_cache: HashMap<(String, String), Entry>
}

impl CredentialManager {
    #[tracing::instrument]
    pub fn new() -> Result<CredentialManager> {
        Ok(CredentialManager {
            connection: CredentialManager::get_connection()?,
            entry_cache: HashMap::new()
        })
    }

    #[tracing::instrument]
    fn clean_url(&self, url: &str) -> String {
        if let Some(url) = url.strip_suffix("/") {
            return url.to_string();
        }
        
        return url.to_string();
    }

    #[tracing::instrument]
    fn get_connection() -> Result<Connection> {
        // Get the path to the credential database
        let mut path = get_config_dir()?;
        path.push("credential_store.db");
        let sqlite_path = path.as_path();

        // Create the folder if it doesn't already exist.
        if !sqlite_path.parent().context("No parent")?.exists(){
            debug!("Creating directories for sqlite database.");
            create_dir_all(sqlite_path.parent().context("No parent")?)?;
        }

        debug!("Creating sqlite database connection.");
        Ok(Connection::open(sqlite_path)?)
    }

    #[tracing::instrument]
    fn get_database_credential_iter(&self, url: &str) -> Result<Vec<DatabaseCredential>> {
        let database = self.get_database()?;

        info!("Selecting rows from user database.");
        let mut stmt: rusqlite::Statement<'_> = database.prepare(
            "SELECT user, device_id_encrypted, device_id_nonce FROM Credentials WHERE url=:url;")?;
        let rows: Vec<DatabaseCredential> = stmt.query_map(&[(":url", url)], |row| {
            Ok(DatabaseCredential {
                user: row.get(0)?,
                device_id_encrypted: row.get(1)?,
                device_id_nonce: row.get(2)?
            })
        })?.filter_map(|r| r.ok()).collect::<Vec<DatabaseCredential>>();

        debug!(count=rows.len(), "Found user rows.");
        Ok(rows)
    }

    #[tracing::instrument]
    fn create_tables(&self, connection: &Connection) -> Result<()> {
        connection.execute(
            "CREATE TABLE IF NOT EXISTS Credentials (
                id                      INTEGER PRIMARY KEY,
                url                     TEXT NOT NULL,
                user                    TEXT NOT NULL,
                device_id_encrypted     BLOB,
                device_id_nonce         BLOB
            )",
            (), // empty list of parameters.
        )?;
        
        connection.execute(
            "CREATE TABLE IF NOT EXISTS Metadata (
                id                      INTEGER PRIMARY KEY,
                key                     TEXT NOT NULL,
                value                   TEXT NOT NULL
            )",
            (), // empty list of parameters.
        )?;

        connection.execute(
            "INSERT INTO Metadata (key, value) VALUES (?1, ?2)",
            (
                "version",
                1.to_string()
            )
        )?;

        Ok(())
    }

    #[tracing::instrument]
    fn get_database(&self) -> Result<&Connection> {
        info!("Creating Credentials table in user database.");

        let conn = &self.connection;
        let version = self.get_database_version(&conn);

        match version {
            Ok(version) => {
                match version {
                    0 => {
                        conn.execute(
                            "ALTER TABLE Credentials DROP COLUMN totp_command_encrypted;
                             ALTER TABLE Credentials DROP COLUMN totp_nonce;
                             ALTER TABLE Credentials ADD COLUMN device_id_encrypted BLOB;
                             ALTER TABLE Credentials ADD COLUMN device_id_nonce BLOB;",
                            (), // empty list of parameters.
                        )?;

                        self.create_tables(&conn)
                    }
                    1 => {
                        // Up to date. Do Nothing

                        Ok(())
                    }
                    _ => bail!("The version is unknown.")
                }
            },
            Err(err) =>
                match err {
                    CredentialError::DatabaseNotInitialized => {
                        self.create_tables(&conn)
                    },
                    _ => Err(anyhow!(err))
                }
        }?;

        Ok(conn)
    }

    #[tracing::instrument]
    fn get_database_version(&self, connection: &Connection) -> Result<u32, CredentialError> {
        info!("Selecting rows from sqlite_master table.");
        let mut stmt: rusqlite::Statement<'_> = connection.prepare(
            "SELECT name FROM sqlite_master WHERE type='table' AND name IN ('Credentials', 'Metadata');")?;
        let rows = stmt.query_map([], |row| {
            Ok(row.get(0)?)
        })?.filter_map(|r| r.ok()).collect::<Vec<String>>();
        
        if rows.is_empty() {
            return Err(CredentialError::DatabaseNotInitialized);
        }
        else if rows.len() == 1 {
            // There is no Metadata table and thus no version.
            return Ok(0);
        }
        
        let mut stmt: rusqlite::Statement<'_> = connection.prepare(
            "SELECT value FROM Metadata WHERE key='version'")?;
        let rows = stmt
            .query_map([], |row| row.get(0))?
            .filter_map(|r| r.ok())
            .map(|r: String| r.parse::<u32>())
            .filter_map(|r| r.ok())
            .collect::<Vec<u32>>();
        let version = rows
            .first()
            .context("Query should return at least 1 item.")?;

        Ok(version.to_owned())
    }

    #[tracing::instrument]
    fn get_entry(&mut self, url: &str, user: &str) -> Result<&Entry>{
        if let std::collections::hash_map::Entry::Vacant(e) = self.entry_cache.entry((url.to_string(), user.to_string())) {
            debug!(user=user, url=url, "Entry did not exist in cache.");

            info!("Creating entry.");
            let entry = Entry::new(url, user)?;
            e.insert(entry);
        }

        info!("Returning entry from cache.");
        self.entry_cache.get(&(url.to_string(), user.to_string())).context("Entry does not exist in cache")
    }

    #[tracing::instrument]
    fn pad_string(&self, input: &str) -> String {
        let mut output = input.to_string();

        while output.len() < 32 {
            output.push(' ');
        }

        output
    }

    #[tracing::instrument]
    pub fn get_credential(&mut self, url: &str) -> Result<Option<Credential>> {
        let url_string = self.clean_url(url);
        let url= url_string.as_str();

        if !self.has_credential(url)? {
            debug!(url=url, "Entry did not exist in sqlite database.");
            return Ok(None);
        }

        info!("Getting entry from sqlite database.");
        let database_rows = self.get_database_credential_iter(url)?;
        let database_credential = database_rows.first().context("No elements returned from database.")?;

        info!("Getting password from operating system credential store.");
        let entry = self.get_entry(url, &database_credential.user)?;
        let password = entry.get_password()?;

        let mut device_id: Option<String> = None;
        if let Some(nonce_vec) = database_credential.device_id_nonce.clone() {
            info!("Database has device id, decrypting.");

            let padded_password = self.pad_string(password.as_str());
            let key: &Key<Aes256Gcm> = padded_password.as_bytes().into();

            let cipher = Aes256Gcm::new(key);
            let nonce = Nonce::from_iter(nonce_vec);
            let plaintext = cipher.decrypt(
                &nonce, 
                database_credential.device_id_encrypted.clone().context("Device ID is empty.")?.as_ref())?;
                device_id = Some(String::from_utf8(plaintext)?);

            info!("Decryption completed.")
        }

        let mut credential = Credential::new(database_credential.user.clone(), password);
        credential.device_id = device_id;

        Ok(Some(credential))
    }

    #[tracing::instrument]
    pub fn has_credential(&self, url: &str) -> Result<bool> {
        let url_string = self.clean_url(url);
        let url= url_string.as_str();

        let database_rows = self.get_database_credential_iter(url)?;

        Ok(!database_rows.is_empty())
    }

    #[tracing::instrument]
    pub fn remove_credential(&mut self, url: &str) -> Result<()> {
        let url_string = self.clean_url(url);
        let url= url_string.as_str();

        if self.has_credential(url)? {
            debug!(url=url, "Entry found in sqlite database.");

            let database_rows = self.get_database_credential_iter(url)?;
            let database_credential = database_rows.first().context("No elements returned from database.")?;

            info!("Removing entry from operating system credential store.");
            let entry = self.get_entry(url, &database_credential.user)?;
            entry.delete_credential()?;
            self.entry_cache.remove(&(url.to_string(), database_credential.user.to_string()));

            info!("Removing entry from sqlite database.");
            let database = self.get_database()?;

            database.execute(
                "DELETE FROM Credentials WHERE url=?1",
                [url].map(|n| n.to_string()),
            )?;
        }

        Ok(())
    }

    #[tracing::instrument]
    pub fn set_credential(&mut self, url: &str, credential: &Credential) -> Result<()> {
        let url_string = self.clean_url(url);
        let url= url_string.as_str();

        if self.has_credential(url)? {
            debug!("Credential exists already.  Removing it before continuing.");
            self.remove_credential(url)?;
        }

        let mut device_id_encrypted: Option<Vec<u8>> = None;
        let mut device_id_nonce: Option<Vec<u8>> = None;
        if let Some(device_id) = credential.device_id.clone() {
            info!("Encrypting the device id.");
            let padded_password = self.pad_string(credential.password.as_str());
            let key: &Key<Aes256Gcm> = padded_password.as_bytes().into();

            let cipher = Aes256Gcm::new(key);
            let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits; unique per message
            let ciphertext = cipher.encrypt(&nonce, device_id.as_bytes())?;

            device_id_encrypted = Some(ciphertext);
            device_id_nonce = Some(nonce.as_slice().to_vec());

            info!("Finished encrypting the device id.");
        }

        info!("Storing credential into database.");
        let database = self.get_database()?;
        database.execute(
            "INSERT INTO Credentials (url, user, device_id_encrypted, device_id_nonce) VALUES (?1, ?2, ?3, ?4)",
            (
                url.to_string(),
                credential.user.to_string(),
                device_id_encrypted,
                device_id_nonce,
        ))?;

        info!("Storing the database into the operating system credential store.");
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

    use super::{Credential, CredentialError, CredentialManager};

    fn create_version_0_database(connection: &Connection) -> anyhow::Result<()> {
        connection.execute(
            "CREATE TABLE IF NOT EXISTS Credentials (
                id                      INTEGER PRIMARY KEY,
                url                     TEXT NOT NULL,
                user                    TEXT NOT NULL,
                totp_command_encrypted  BLOB,
                totp_nonce              BLOB
            )",
            (), // empty list of parameters.
        )?;

        Ok(())
    }

    fn create_version_1_database(connection: &Connection) -> anyhow::Result<()> {
        connection.execute(
            "CREATE TABLE IF NOT EXISTS Credentials (
                id                      INTEGER PRIMARY KEY,
                url                     TEXT NOT NULL,
                user                    TEXT NOT NULL,
                device_id_encrypted     BLOB,
                device_id_nonce         BLOB
            )",
            (), // empty list of parameters.
        )?;
        
        connection.execute(
            "CREATE TABLE IF NOT EXISTS Metadata (
                id                      INTEGER PRIMARY KEY,
                key                     TEXT NOT NULL,
                value                   TEXT NOT NULL
            )",
            (), // empty list of parameters.
        )?;

        connection.execute(
            "INSERT INTO Metadata (key, value) VALUES (?1, ?2)",
            (
                "version",
                1.to_string()
            )
        )?;

        Ok(())
    }

    fn new_credential(user: &str, password: &str, device_id: Option<&str>) -> Credential {
        let mut credential = Credential::new(user.to_string(), password.to_string());

        if let Some(device_id) = device_id {
            credential.device_id = Some(device_id.to_string());
        }

        credential
    }

    #[test]
    fn create_credential_manager() {
        let _ = CredentialManager::new();
    }

    #[test]
    fn empty_database_uninitialized() {
        let credential_manager = CredentialManager {
            connection: Connection::open_in_memory().unwrap(),
            entry_cache: HashMap::new()
        };

        let result = credential_manager.get_database_version(&credential_manager.connection);
        
        match result {
            Ok(_) => panic!(),
            Err(err) => match err {
                CredentialError::DatabaseNotInitialized => (),
                _ => Err(err).unwrap()
            }
        }
    }

    #[test]
    fn get_database_version_0() {
        let connection = Connection::open_in_memory().unwrap();
        create_version_0_database(&connection).unwrap();

        let credential_manager = CredentialManager {
            connection,
            entry_cache: HashMap::new()
        };

        let version = credential_manager.get_database_version(&credential_manager.connection).unwrap();

        assert_eq!(version, 0);
    }

    #[test]
    fn get_database_version_1() {
        let connection = Connection::open_in_memory().unwrap();
        create_version_1_database(&connection).unwrap();

        let credential_manager = CredentialManager {
            connection,
            entry_cache: HashMap::new()
        };

        let version = credential_manager.get_database_version(&credential_manager.connection).unwrap();

        assert_eq!(version, 1);
    }

    #[test]
    fn database_upgraded_from_0_to_1() {
        let connection = Connection::open_in_memory().unwrap();
        create_version_0_database(&connection).unwrap();

        let credential_manager = CredentialManager {
            connection,
            entry_cache: HashMap::new()
        };

        let connection = credential_manager.get_database().unwrap();
        let version = credential_manager.get_database_version(connection).unwrap();
        
        assert_eq!(version, 1);
    }

    #[test]
    fn set_get_credential_no_device_id() {
        set_default_credential_builder(mock::default_credential_builder()); // Set mock

        let mut credential_manager = CredentialManager {
            connection: Connection::open_in_memory().unwrap(),
            entry_cache: HashMap::new()
        };

        credential_manager.set_credential("http://example.com", &&new_credential("test_user", "test_password", None)).unwrap();

        let credential: Credential = credential_manager.get_credential("http://example.com").unwrap().context("Credential expected").unwrap();

        assert_eq!(credential.user, "test_user".to_string());
        assert_eq!(credential.password, "test_password".to_string());
        assert_eq!(credential.device_id, None);
    }

    #[test]
    fn set_get_credential_no_device_id_with_slash() {
        set_default_credential_builder(mock::default_credential_builder()); // Set mock

        let mut credential_manager = CredentialManager {
            connection: Connection::open_in_memory().unwrap(),
            entry_cache: HashMap::new()
        };

        credential_manager.set_credential("http://example.com/", &&new_credential("test_user", "test_password", None)).unwrap();

        let credential: Credential = credential_manager.get_credential("http://example.com").unwrap().context("Credential expected").unwrap();

        assert_eq!(credential.user, "test_user".to_string());
        assert_eq!(credential.password, "test_password".to_string());
        assert_eq!(credential.device_id, None);
    }

    #[test]
    fn set_get_credential_device_id() {
        set_default_credential_builder(mock::default_credential_builder()); // Set mock

        let mut credential_manager = CredentialManager {
            connection: Connection::open_in_memory().unwrap(),
            entry_cache: HashMap::new()
        };

        credential_manager.set_credential("http://example.com", &&new_credential("test_user", "test_password", Some("12345"))).unwrap();

        let credential: Credential = credential_manager.get_credential("http://example.com").unwrap().context("Credential expected").unwrap();

        assert_eq!(credential.user, "test_user".to_string());
        assert_eq!(credential.password, "test_password".to_string());
        assert_eq!(credential.device_id.context("Should not be null").unwrap(), "12345".to_string());
    }

    #[test]
    fn has_credential() {
        set_default_credential_builder(mock::default_credential_builder()); // Set mock

        let mut credential_manager = CredentialManager {
            connection: Connection::open_in_memory().unwrap(),
            entry_cache: HashMap::new()
        };

        credential_manager.set_credential("http://example.com", &&new_credential("test_user", "test_password", Some("echo 12345"))).unwrap();

        assert!(credential_manager.has_credential("http://example.com").unwrap());
    }

    #[test]
    fn can_update_credential_no_device_id() {
        set_default_credential_builder(mock::default_credential_builder()); // Set mock

        let mut credential_manager = CredentialManager {
            connection: Connection::open_in_memory().unwrap(),
            entry_cache: HashMap::new()
        };

        credential_manager.set_credential("http://example.com", &new_credential("test_user", "test_password", Some("echo 12345"))).unwrap();

        credential_manager.set_credential("http://example.com", &new_credential("test_user", "test_password2", None)).unwrap();

        let credential: Credential = credential_manager.get_credential("http://example.com").unwrap().context("Credential expected").unwrap();

        assert_eq!(credential.user, "test_user".to_string());
        assert_eq!(credential.password, "test_password2".to_string());
        assert_eq!(credential.device_id, None);
    }

    #[test]
    fn can_update_credential_device_id() {
        set_default_credential_builder(mock::default_credential_builder()); // Set mock

        let mut credential_manager = CredentialManager {
            connection: Connection::open_in_memory().unwrap(),
            entry_cache: HashMap::new()
        };

        credential_manager.set_credential("http://example.com", &new_credential("test_user", "test_password", Some("12345"))).unwrap();

        credential_manager.set_credential("http://example.com", &new_credential("test_user", "test_password2", Some("56789"))).unwrap();

        let credential: Credential = credential_manager.get_credential("http://example.com").unwrap().context("Credential expected").unwrap();

        assert_eq!(credential.user, "test_user".to_string());
        assert_eq!(credential.password, "test_password2".to_string());
        assert_eq!(credential.device_id.context("TOTP should of not null").unwrap(), "56789".to_string());
    }

    #[test]
    fn remove_credential() {
        set_default_credential_builder(mock::default_credential_builder()); // Set mock

        let mut credential_manager = CredentialManager {
            connection: Connection::open_in_memory().unwrap(),
            entry_cache: HashMap::new()
        };

        credential_manager.set_credential("http://example.com", &new_credential("test_user", "test_password", Some("echo 12345"))).unwrap();
        credential_manager.remove_credential("http://example.com").unwrap();

        assert!(!credential_manager.has_credential("http://example.com").unwrap());
    }
}