use num_traits::FromPrimitive;
use reqwest::{Error, Response, StatusCode};
use serde::de::DeserializeOwned;
use tracing::{info, warn};

use crate::credential_manager::Credential;

use super::responses::{LoginError, LoginResult, SynologyEmptyError, SynologyError, SynologyErrorStatus, SynologyResult, SynologyStatusCode};

#[derive(Debug)]
pub struct SynologyFileStation {
    sid: Option<String>,
    url: String,
    version: u8
}

impl SynologyFileStation {
    #[tracing::instrument]
    pub fn new(url: &str) -> SynologyFileStation {
        SynologyFileStation::new_with_version(url, 7)
    }

    #[tracing::instrument]
    pub fn new_with_version(url: &str, version: u8) -> SynologyFileStation {
        SynologyFileStation {
            sid: None,
            url: url.to_string(),
            version
        }
    }

    #[tracing::instrument]
    async fn parse_data_and_error<TData: DeserializeOwned, TError: DeserializeOwned>(&self, response: Result<Response, Error>) -> Result<(Option<TData>, Option<SynologyError<TError>>), SynologyErrorStatus> {
        match response {
            Ok(response) => {
                match response.status() {
                    StatusCode::OK => {
                        match response.text().await {
                            Ok(text) => {
                                info!("Parsing response from server.");
                                let result = serde_json::from_str::<SynologyResult<TData, TError>>(text.as_str());

                                match result {
                                    Ok(result) => Ok((result.data, result.error)),
                                    Err(error) => Err(SynologyErrorStatus::SerdeError(error))
                                }
                            },
                            Err(error) => Err(SynologyErrorStatus::ReqwestError(error))
                        }
                    },
                    _ => Err(SynologyErrorStatus::HttpError(response.status()))
                }
            },
            Err(error) =>
                match error.status() {
                    Some(status) => Err(SynologyErrorStatus::HttpError(status)),
                    None => Err(SynologyErrorStatus::UnknownError)
                }
        }
    }

    #[tracing::instrument]
    async fn parse<T: DeserializeOwned>(&self, response: Result<Response, Error>) -> Result<T, SynologyErrorStatus> {
        let (data, error) = self.parse_data_and_error::<T, SynologyEmptyError>(response).await?;

        match error {
            Some(error) => {
                info!("A server error occurred");

                match FromPrimitive::from_u32(error.code) {
                    Some(code) => Err(SynologyErrorStatus::ServerError(code)),
                    None => Err(SynologyErrorStatus::UnknownError)
                }
            },
            None => Ok(())
        }?;

        match data {
            Some(data) => Ok(data),
            None => {
                warn!("No data and no error from server");

                Err(SynologyErrorStatus::UnknownError)
            }
        }
    }

    #[tracing::instrument]
    pub async fn login(&mut self, credential: &Credential) -> Result<(), SynologyErrorStatus> {
        let login_url = format!(
            "{}/webapi/entry.cgi?api=SYNO.API.Auth&version={}&method=login&account={}&passwd={}&session=FileStation&fromat=sid",
            self.url,
            self.version,
            credential.user,
            credential.password
        );

        // Make initial request to the server.  This will fail if the user needs a TOTP.
        let response = reqwest::get(login_url).await;
        let (mut login_result, login_error) = self.parse_data_and_error::<LoginResult, LoginError>(response).await?;

        match login_error {
            Some(login_error) => 
                match FromPrimitive::from_u32(login_error.code) {
                    Some(code) =>
                        match code {
                            SynologyStatusCode::InvalidUserDoesThisFileOperation => {
                                info!("Server sent back auth error.  We may need to ask the user for a TOTP.");

                                match login_error.errors {
                                    Some(errors) =>
                                        if errors.types.iter().any(|f| f.contains_key("type") && f["type"] == "otp") {
                                            info!("Server requested TOTP");
                                            let totp = credential.totp();

                                            match totp {
                                                Some(totp) => {
                                                    info!("Requested TOTP from TOTP command");

                                                    let login_url = format!(
                                                        "{}/webapi/entry.cgi?api=SYNO.API.Auth&version={}&method=login&account={}&passwd={}&session=FileStation&fromat=sid&otp_code={}",
                                                        self.url,
                                                        self.version,
                                                        credential.user,
                                                        errors.token,
                                                        totp
                                                    );

                                                    let response = reqwest::get(login_url).await;
                                                    login_result= Some(self.parse::<LoginResult>(response).await?);

                                                    Ok(())
                                                },
                                                None => Err(SynologyErrorStatus::NoTotp)
                                            }
                                        }
                                        else {
                                            Err(SynologyErrorStatus::ServerError(SynologyStatusCode::InvalidUserDoesThisFileOperation))
                                        }
                                    None => Err(SynologyErrorStatus::UnknownError)
                                }
                            }
                            _ => Err(SynologyErrorStatus::ServerError(code))
                        },
                    None => Err(SynologyErrorStatus::UnknownError)
                },
            None => Ok(())
        }?;

        match login_result {
            Some(login_result) => {
                self.sid = Some(login_result.sid);

                Ok(())
            },
            None => Err(SynologyErrorStatus::UnknownError)
        }
    }
}