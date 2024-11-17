use std::fs::create_dir_all;

use aes_gcm::{aead::{Aead, OsRng}, AeadCore, Aes256Gcm, Key, KeyInit, Nonce};
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
    totp_nonce: Option<Vec<u8>>
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
            println!("Creating table");
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
        }

        Ok(conn)
    }

    fn get_database_credential_iter(&self, url: &str) -> Result<Vec<DatabaseCredential>> {
        let database = self.get_database()?;

        let mut stmt: rusqlite::Statement<'_> = database.prepare(
            "SELECT id, url, user, totp_command_encrypted, totp_nonce FROM Credentials WHERE url=:url;")?;
        let rows: Vec<DatabaseCredential> = stmt.query_map(&[(":url", url)], |row| {
            Ok(DatabaseCredential {
                id: row.get(0)?,
                url: row.get(1)?,
                user: row.get(2)?,
                totp_comand_encrypted: row.get(3)?,
                totp_nonce: row.get(4)?
            })
        })?.filter_map(|r| r.ok()).collect::<Vec<DatabaseCredential>>().try_into()?;

        drop(stmt); // Allow closing the database.

        match database.close() {
            Ok(_) => Ok(rows),
            Err(_) => Err(anyhow::Error::msg("An error occurred closig the database."))
        }
    }

    pub fn get_credential(&self, url: &str) -> Result<Credential> {
        let database_rows = self.get_database_credential_iter(url)?;
        let database_credential = database_rows.first().context("No elements returned from database.")?;
        let entry = Entry::new(url, &database_credential.user)?;

        let password = entry.get_password()?;
        let mut totp_command: Option<String> = None;

        if database_credential.totp_comand_encrypted.is_some() {
            let key: &Key<Aes256Gcm> = password.as_bytes().into();

            let nonce_vec = database_credential.totp_nonce.clone().context("No nonce provided for credential")?;

            let cipher = Aes256Gcm::new(&key);
            let nonce = Nonce::from_iter(nonce_vec);
            let plaintext = cipher.decrypt(
                &nonce, 
                database_credential.totp_comand_encrypted.clone().context("TOTP command is empty.")?.as_ref())?;
            totp_command = Some(String::from_utf8(plaintext)?);
        }

        Ok(Credential::new(&database_credential.user, &password, totp_command))
    }

    pub fn has_credential(&self, url: &str) -> Result<bool> {
        let database_rows = self.get_database_credential_iter(url)?;

        Ok(!database_rows.is_empty())
    }

    pub fn remove_credential(&self, url: &str) -> Result<()> {
        if self.has_credential(url)? {
            let database_rows = self.get_database_credential_iter(url)?;
            let database_credential = database_rows.first().context("No elements returned from database.")?;

            let entry = Entry::new(&database_credential.url, &database_credential.user)?;
            entry.delete_credential()?;

            let database = self.get_database()?;

            database.execute(
                "DELETE FROM Credentials WHERE url=?1",
                [url].map(|n| n.to_string()),
            )?;
        }

        Ok(())
    }

    pub fn set_credential(&self, url: &str, credential: &Credential) -> Result<()> {
        if self.has_credential(url)? {
            self.remove_credential(url)?;
        }

        let mut totp_comand_encrypted: Option<Vec<u8>> = None;
        let mut totp_nonce: Option<Vec<u8>> = None;
        if credential.totp_command.is_some() {
            let key: &Key<Aes256Gcm> = credential.password.as_bytes().into();

            let cipher = Aes256Gcm::new(&key);
            let nonce = Aes256Gcm::generate_nonce(&mut OsRng); // 96-bits; unique per message
            let ciphertext = cipher.encrypt(&nonce, credential.totp_command.clone().context("TOTP Command does not exist.")?.as_bytes())?;

            totp_comand_encrypted = Some(ciphertext);
            totp_nonce = Some(nonce.as_slice().to_vec());
        }

        let database = self.get_database()?;
        database.execute(
            "INSERT INTO Credential (url, user, totp_command_encrypted, totp_nonce) VALUES (?1)",
            (url.to_string(), credential.user.to_string(), totp_comand_encrypted, totp_nonce),
        )?;

        let entry = Entry::new(url, &credential.user)?;
        entry.set_password(&credential.password)?;
        
        match database.close() {
            Ok(_) => Ok(()),
            Err(_) => Err(anyhow::Error::msg("An error occurred closig the database."))
        }
    }
}