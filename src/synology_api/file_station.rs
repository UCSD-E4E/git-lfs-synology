use num_traits::FromPrimitive;
use reqwest::{Error, Response, StatusCode};
use serde::de::DeserializeOwned;
use tracing::info;

use crate::credential_manager::Credential;

use super::responses::{LoginResult, SynologyError, SynologyPartial, SynologyResponseError, SynologyResult};

#[derive(Debug)]
pub struct SynologyFileStation {
    sid: Option<String>,
    url: String,
    version: u8
}

impl SynologyFileStation {
    pub fn new(url: &str) -> SynologyFileStation {
        SynologyFileStation::new_with_version(url, 7)
    }

    pub fn new_with_version(url: &str, version: u8) -> SynologyFileStation {
        SynologyFileStation {
            sid: None,
            url: url.to_string(),
            version
        }
    }

    #[tracing::instrument]
    async fn parse<T: DeserializeOwned>(&self, response: Result<Response, Error>) -> Result<T, SynologyError> {
        match response {
            Ok(response) => {
                match response.status() {
                    StatusCode::OK => {
                        match response.text().await {
                            Ok(text) => {
                                let result = serde_json::from_str::<SynologyPartial>(text.as_str());

                                match result {
                                    Ok(result) => {
                                        if result.success {
                                            let result = serde_json::from_str::<SynologyResult<T>>(text.as_str());

                                            match result {
                                                Ok(result) => Ok(result.data),
                                                Err(error) => Err(SynologyError::SerdeError(error))
                                            }
                                        }
                                        else {
                                            let result = serde_json::from_str::<SynologyResult<SynologyResponseError>>(text.as_str());

                                            match result {
                                                Ok(result) => 
                                                    match FromPrimitive::from_u32(result.data.code) {
                                                        Some(code) => Err(SynologyError::ServerError(code)),
                                                        None => Err(SynologyError::UnknownError)
                                                    },
                                                Err(error) => Err(SynologyError::SerdeError(error))
                                            }
                                        }
                                    },
                                    Err(error) => Err(SynologyError::SerdeError(error))
                                }
                            },
                            Err(error) => Err(SynologyError::ReqwestError(error))
                        }
                    },
                    _ => Err(SynologyError::HttpError(response.status()))
                }
            },
            Err(error) =>
                match error.status() {
                    Some(status) => Err(SynologyError::HttpError(status)),
                    None => Err(SynologyError::UnknownError)
                }
        }
    }

    #[tracing::instrument]
    pub async fn login(&mut self, credential: &Credential) -> Result<(), SynologyError> {
        let totp = credential.totp();

        let mut login_url = format!(
            "{}/webapi/auth.cgi?api=SYNO.API.Auth&version={}&method=login&account={}&passwd={}&session=FileStation&fromat=sid",
            self.url,
            self.version,
            credential.user,
            credential.password
        );

        match totp {
            Some(totp) => {
                info!("Using TOTP for login.");
                login_url = format!(
                    "{}&otp={}",
                    login_url, totp
                );
            },
            None => {}
        };

        let response = reqwest::get(login_url).await;
        let login_result: LoginResult = self.parse(response).await?;

        self.sid = Some(login_result.sid);

        Ok(())
    }
}