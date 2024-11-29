use std::{collections::HashMap, io};

use futures_util::{StreamExt, TryFutureExt};
use num_traits::FromPrimitive;
use reqwest::{Error, Response, StatusCode};
use serde::de::DeserializeOwned;
use tokio::fs::File;
use tokio_util::io::ReaderStream;
use tracing::{info, warn};
use urlencoding::encode;

use crate::credential_manager::Credential;

use super::{responses::{CreateFolderResult, LoginError, LoginResult, SynologyError, SynologyErrorStatus, SynologyResult, SynologyStatusCode}, ProgressReporter};

#[derive(Clone, Debug)]
pub struct SynologyFileStation {
    sid: Option<String>,
    url: String
}

impl SynologyFileStation {
    #[tracing::instrument]
    pub fn new(url: &str) -> SynologyFileStation {
        SynologyFileStation {
            sid: None,
            url: url.to_string()
        }
    }

    #[tracing::instrument]
    async fn get<T: DeserializeOwned>(&self, api: &str, method: &str, version: u32, parameters: &HashMap<&str, &str>) -> Result<T, SynologyErrorStatus> {
        match &self.sid {
            Some(sid) => {
                info!("Found sid, continuing.");
                let mut url = format!(
                    "{}/webapi/entry.cgi?api={}&version={}&method={}&_sid={}",
                    self.url,
                    api,
                    version,
                    method,
                    sid.as_str()
                );

                for (key, value) in parameters {
                    url = format!(
                        "{}&{}={}",
                        url,
                        key,
                        encode(value)
                    );
                }

                info!("Get: \"{}\".", url);

                let response = reqwest::get(url).await;
                Ok(self.parse(response).await?)
            },
            None => {
                info!("No sid found. Not logged in");

                Err(SynologyErrorStatus::NotLoggedIn)
            }
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
                                info!("Parsing response from server. Response was \"{}\".", text);
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
        let (data, error) = self.parse_data_and_error::<T, Vec<HashMap<String, String>>>(response).await?;

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

    pub async fn create_folder(&self, folder_path: &str, name: &str, force_parent: bool) -> Result<CreateFolderResult, SynologyErrorStatus> {
        let force_parent_string = force_parent.to_string();

        let mut parameters = HashMap::<&str, &str>::new();
        parameters.insert("folder_path", folder_path);
        parameters.insert("name", name);
        parameters.insert("force_parent", force_parent_string.as_str());

        Ok(self.get("SYNO.FileStation.CreateFolder", "create", 2, &parameters).await?)
    }

    #[tracing::instrument]
    pub async fn login(&mut self, credential: &Credential) -> Result<(), SynologyErrorStatus> {
        let login_url = format!(
            "{}/webapi/entry.cgi?api=SYNO.API.Auth&version={}&method=login&account={}&passwd={}&session=FileStation&fromat=sid",
            self.url,
            6,
            credential.user,
            encode(credential.password.as_str()) // Encode the password in case it has characters not allowed in URLs in it.
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
                                                        6,
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

    pub async fn upload<TProgressReporter: ProgressReporter>(&self,
        file: File,
        path: &str,
        create_parents: bool,
        overwrite: bool,
        mtime: Option<u64>,
        crtime: Option<u64>,
        atime: Option<u64>,
        mut progress_reporter: Option<&'static mut TProgressReporter>
    ) -> Result<(), SynologyErrorStatus> {
        const BYTES_TO_KB: usize = 1024;
        const KB_TO_MB: usize = 1024;
        const BYTES_TO_MB: usize = BYTES_TO_KB * KB_TO_MB;
        const CHUNK_SIZE: usize = 4 * BYTES_TO_MB;

        match &self.sid {
            Some(sid) => {
                let mut reader_stream = ReaderStream::new(file);
                let async_stream = async_stream::stream! {
                    let mut bytes_so_far = 0;

                    while let Some(chunk) = reader_stream.next().await {
                        if let Ok(chunk) = &chunk {
                            bytes_so_far += chunk.len();

                            if let Some(reporter) = &mut progress_reporter {
                                reporter.update(bytes_so_far);
                            }
                        }
                        yield chunk;
                    }
                };

                let url = format!(
                    "{}/webapi/entry.cgi?api={}&version={}&method={}&_sid={}&path={}&create_parents={}&overwrite={}",
                    self.url,
                    "SYNO.FileStation.Upload",
                    2,
                    "upload",
                    sid,
                    path,
                    create_parents,
                    overwrite
                );

                let url = match mtime {
                    Some(mtime) => format!(
                        "{}&mtime={}",
                        url,
                        mtime
                    ),
                    None => url
                };

                let url = match crtime {
                    Some(crtime) => format!(
                        "{}&crtime={}",
                        url,
                        crtime
                    ),
                    None => url
                };

                let url = match atime {
                    Some(atime) => format!(
                        "{}&atime={}",
                        url,
                        atime
                    ),
                    None => url
                };

                let _ = reqwest::Client::new()
                    .post(url)
                    .header("content-type", "application/octet-stream")
                    .body(reqwest::Body::wrap_stream(async_stream))
                    .send()
                    .await?;

                Ok(())
            },
            None => Err(SynologyErrorStatus::NotLoggedIn)
        }
    }
}