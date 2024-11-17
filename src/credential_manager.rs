use keyring::{Entry, Result};

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
    fn delete_entry_keyring(&self, url: &str) -> Result<()> {
        let entry = self.get_entry_keyring(url)?;
        entry.delete_credential()?;

        Ok(())
    }

    fn get_entry_keyring(&self, url: &str) -> Result<Entry> {
        let user = "";

        Entry::new(url, user)
    }

    fn get_password(&self, url: &str) -> Result<String> {
        let entry = self.get_entry_keyring(url)?;

        Ok(entry.get_password()?)
    }

    fn set_entry_keyring(&self, url: &str, user: &str, password: &str) -> Result<()> {
        let entry = Entry::new(url, &user)?;
        entry.set_password(&password)?;

        Ok(())
    }

    pub fn get_credential(&self, url: &str) -> Option<Credential> {
        match self.get_password(url) {
            Ok(password) => {
                let user = "";
                let totp_command = Some("");

                Some(Credential::new(user, password.as_str(), None))
            },
            Err(error) => None
        }
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