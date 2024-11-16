pub struct Credential {
    // user: String,
    // password: String,
    // totp_command: String
}

pub struct CredentialManager {
}

impl CredentialManager {
    pub fn get_credential(&self, url: &str) -> Credential {
        Credential {
            // user: "",
            // password: ""
        }
    }

    pub fn has_credential(&self, url: &str) -> bool {
        true
    }

    pub fn remove_credential(&self, url: &str) {

    }

    pub fn set_credential(&self, url: &str, credential: &Credential) {

    }
}