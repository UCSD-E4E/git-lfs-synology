use std::{collections::HashMap, path::{Path, PathBuf}};

use num_traits::FromPrimitive;
use reqwest::{Error, Response, StatusCode};
use serde::de::DeserializeOwned;
use tokio::{fs::File, io::AsyncWriteExt};
use tracing::{info, warn};
use urlencoding::encode;

use crate::credential_manager::Credential;

use super::{responses::{CreateFolderResponse, ListResponse, ListShareResponse, LoginError, LoginResponse, SynologyError, SynologyErrorStatus, SynologyResult, SynologyStatusCode}, ProgressReporter};

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
    async fn get<T: DeserializeOwned>(&self, api: &str, method: &str, version: u32, parameters: &HashMap<&str, String>) -> Result<T, SynologyErrorStatus> {
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
                                    Ok(result) => {
                                        info!("Successfully parsed response from server.");

                                        Ok((result.data, result.error))
                                    },
                                    Err(error) => {
                                        warn!("An error occurred while trying to process the results. \"{}\".", error);

                                        Err(SynologyErrorStatus::SerdeError(error))
                                    }
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
                warn!("A server error occurred, {}.", error.code);

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
    pub async fn create_folder(&self, folder_path: &str, name: &str, force_parent: bool) -> Result<CreateFolderResponse, SynologyErrorStatus> {
        let force_parent_string = force_parent.to_string();

        let mut parameters = HashMap::<&str, String>::new();
        parameters.insert("folder_path", folder_path.to_string());
        parameters.insert("name", name.to_string());
        parameters.insert("force_parent", force_parent_string);

        self.get("SYNO.FileStation.CreateFolder", "create", 2, &parameters).await
    }

    #[tracing::instrument]
    pub async fn download<TProgressReporter: ProgressReporter + 'static>(
        &self,
        source_file_path: &str,
        target_directory_path: &Path,
        mut progress_reporter: Option<TProgressReporter>) -> Result<PathBuf, SynologyErrorStatus> {
            match &self.sid {
                Some(sid) => {
                    info!("Found sid, continuing.");
                    match source_file_path.split("/").last() {
                        Some(file_name) => {
                            info!("Found file name: \"{}\".", file_name);
                            let url = format!(
                                "{}/webapi/entry.cgi?api={}&version={}&method={}&_sid={}&path={}&mode=download",
                                self.url,
                                "SYNO.FileStation.Download",
                                2,
                                "download",
                                sid.as_str(),
                                source_file_path
                            );
                            info!("Get: \"{}\".", url);
        
                            let mut target_file_path = PathBuf::new();
                            target_file_path.push(target_directory_path);
                            target_file_path.push(file_name);

                            info!("Target File Path: \"{}\".", target_file_path.as_os_str().to_string_lossy());
        
                            let mut target_stream = File::create(&target_file_path).await?;
                            let mut response = reqwest::get(url).await?;

                            while let Ok(chunk) = response.chunk().await {
                                if let Some(chunk) = chunk {
                                    let write_len = target_stream.write(&chunk).await?;

                                    if let Some(progress_reporter) = &mut progress_reporter {
                                        let result = progress_reporter.update(write_len);

                                        if let Err(error) = result {
                                            warn!("An error occurred reporting progress: \"{error}\".");
                                        }
                                    }
                                }
                                else {
                                    break;
                                }
                            }

                            if let Some(mut progress_reporter) = progress_reporter {
                                if let Some(total_bytes) = response.content_length(){
                                    info!("Reporting complete progress.");
                                    let result = progress_reporter.update(total_bytes as usize );
                
                                    if let Err(error) = result {
                                        warn!("An error occurred reporting progress: \"{error}\".");
                                    }
                                }
                            }
                            Ok(target_file_path)
                        },
                        None => Err(SynologyErrorStatus::UnknownError)
                    }
                }
                None => Err(SynologyErrorStatus::NotLoggedIn)
            }
    }

    #[allow(clippy::too_many_arguments)] // Allow this so that we better match the Synology API.
    #[tracing::instrument]
    pub async fn list(
        &self,
        folder_path: &str,
        offset: Option<u64>,
        limit: Option<u64>,
        sort_by: Option<String>,
        sort_direction: Option<String>,
        pattern: Option<String>,
        file_type: Option<String>,
        goto_path: Option<String>,
        include_real_path: bool,
        include_size: bool,
        include_owner: bool,
        include_time: bool,
        include_perm: bool,
        include_mount_point_type: bool,
        include_type: bool
    ) -> Result<ListResponse, SynologyErrorStatus> {
        let mut parameters = HashMap::<&str, String>::new();
        parameters.insert("folder_path", folder_path.to_string());

        if let Some(offset) = offset {
            parameters.insert("offset", offset.to_string());
        }

        if let Some(limit) = limit {
            parameters.insert("limit", limit.to_string());
        }

        if let Some(sort_by) = sort_by {
            parameters.insert("sort_by", sort_by);
        }

        if let Some(sort_direction) = sort_direction {
            parameters.insert("sort_direction", sort_direction);
        }

        if let Some(pattern) = pattern {
            parameters.insert("pattern", pattern);
        }

        if let Some(file_type) = file_type {
            parameters.insert("filetype", file_type);
        }

        if let Some(goto_path) = goto_path {
            parameters.insert("goto_path", goto_path);
        }

        let mut additional: String = String::new();

        if include_real_path {
            additional = format!("{},real_path", additional);
        }

        if include_size {
            additional = format!("{},size", additional);
        }

        if include_owner {
            additional = format!("{},owner", additional);
        }

        if include_time {
            additional = format!("{},time", additional);
        }

        if include_perm {
            additional = format!("{},perm", additional);
        }

        if include_mount_point_type {
            additional = format!("{},mount_point_type", additional);
        }

        if include_type {
            additional = format!("{},type", additional);
        }

        if !additional.is_empty() {
            parameters.insert("additional", additional[1..].to_string());
        }

        self.get("SYNO.FileStation.List", "list", 2, &parameters).await
    }

    #[allow(clippy::too_many_arguments)] // Allow this so that we better match the Synology API.
    #[tracing::instrument]
    pub async fn list_share(
        &self,
        offset: Option<u64>,
        limit: Option<u64>,
        sort_by: Option<String>,
        sort_direction: Option<String>,
        only_writable: Option<bool>,
        include_real_path: bool,
        include_size: bool,
        include_owner: bool,
        include_time: bool,
        include_perm: bool,
        include_mount_point_type: bool,
        include_volume_status: bool
    ) -> Result<ListShareResponse, SynologyErrorStatus> {
        let mut parameters = HashMap::<&str, String>::new();

        if let Some(offset) = offset {
            parameters.insert("offset", offset.to_string());
        }

        if let Some(limit) = limit {
            parameters.insert("limit", limit.to_string());
        }

        if let Some(sort_by) = sort_by {
            parameters.insert("sort_by", sort_by);
        }

        if let Some(sort_direction) = sort_direction {
            parameters.insert("sort_direction", sort_direction);
        }

        if let Some(only_writable) = only_writable {
            parameters.insert("only_writable", only_writable.to_string());
        }

        let mut additional: String = String::new();

        if include_real_path {
            additional = format!("{},real_path", additional);
        }

        if include_size {
            additional = format!("{},size", additional);
        }

        if include_owner {
            additional = format!("{},owner", additional);
        }

        if include_time {
            additional = format!("{},time", additional);
        }

        if include_perm {
            additional = format!("{},perm", additional);
        }

        if include_mount_point_type {
            additional = format!("{},mount_point_type", additional);
        }

        if include_volume_status {
            additional = format!("{},volume_status", additional);
        }

        if !additional.is_empty() {
            parameters.insert("additional", additional[1..].to_string());
        }

        self.get("SYNO.FileStation.List", "list_share", 2, &parameters).await
    }

    #[tracing::instrument]
    pub async fn login(&mut self, credential: &Credential, enable_device_token: bool, totp: Option<String>) -> Result<Credential, SynologyErrorStatus> {
        let device_name = format!(
            "{}::{}",
            hostname::get()?.to_string_lossy(),
            "rust_synology_api"
        );

        let mut login_url = format!(
            "{}/webapi/entry.cgi?api=SYNO.API.Auth&version={}&method=login&account={}&passwd={}&enable_device_token={}&device_name={}&session=FileStation&fromat=sid",
            self.url,
            6,
            encode(credential.user.as_str()),
            encode(credential.password.as_str()), // Encode the password in case it has characters not allowed in URLs in it.
            enable_device_token,
            device_name
        );

        if let Some(did) = credential.device_id.clone() {
            info!("Credential has device ID");

            login_url = format!(
                "{}&device_id={}",
                login_url,
                did
            )
        }

        if let Some(totp) = totp {
            info!("TOTP has been provided.");

            login_url = format!(
                "{}&otp_code={}",
                login_url,
                totp
            )
        }

        // Make initial request to the server.  This will fail if the user needs a TOTP.
        let response = reqwest::get(login_url).await;
        let (login_result, login_error) = self.parse_data_and_error::<LoginResponse, LoginError>(response).await?;

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
                                            
                                            Err(SynologyErrorStatus::NoTotp)
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

                let mut cred = Credential::new(credential.user.to_string(), credential.password.to_string());
                cred.device_id = login_result.did;

                Ok(cred)
            },
            None => Err(SynologyErrorStatus::UnknownError)
        }
    }

    #[allow(clippy::too_many_arguments)] // Allow this so that we better match the Synology API.
    #[tracing::instrument]
    pub async fn upload<TProgressReporter: ProgressReporter + 'static>(&self,
        source_file_path: &Path,
        total_bytes: usize,
        target_directory_path: &str,
        create_parents: bool,
        overwrite: bool,
        mtime: Option<u64>,
        crtime: Option<u64>,
        atime: Option<u64>,
        progress_reporter: Option<TProgressReporter>
    ) -> Result<(), SynologyErrorStatus> {
        match &self.sid {
            Some(sid) => {
                let url = format!(
                    "{}/webapi/entry.cgi?api={}&version={}&method={}&_sid={}",
                    self.url,
                    "SYNO.FileStation.Upload",
                    2,
                    "upload",
                    sid
                );

                info!("Uploading to \"{}\".", url);

                let mut source_path = PathBuf::new();
                source_path.push(source_file_path);
                let source_file_name = match source_path.file_name() {
                    Some(source_file_name) => Ok(source_file_name.to_string_lossy().to_string()),
                    None => Err(SynologyErrorStatus::UnknownError)
                }?;

                let file_path_string = match source_file_path.as_os_str().to_str() {
                    Some(file_path_string) => Ok(file_path_string),
                    None => Err(SynologyErrorStatus::UnknownError)
                }?;

                let part = reqwest::multipart::Part::file(file_path_string)
                    .await?
                    .file_name(source_file_name)
                    .mime_str("application/octet-stream")?;

                let form = reqwest::multipart::Form::new()
                    .text("path", target_directory_path.to_string())
                    .text("create_parents", create_parents.to_string())
                    .text("overwrite", overwrite.to_string())
                    .part("files", part);

                let form = if let Some(mtime) = mtime {
                    form.text("mtime", mtime.to_string())
                }
                else {
                    form
                };

                let form = if let Some(crtime) = crtime {
                    form.text("crtime", crtime.to_string())
                }
                else {
                    form
                };

                let form = if let Some(atime) = atime {
                    form.text("atime", atime.to_string())
                }
                else {
                    form
                };

                let response = reqwest::Client::new()
                    .post(url)
                    .multipart(form)
                    .send()
                    .await;
                let _ = self.parse::<crate::synology_api::responses::Empty>(response).await?;

                if let Some(mut progress_reporter) = progress_reporter {
                    info!("Reporting complete progress.");
                    let result = progress_reporter.update(total_bytes);

                    if let Err(error) = result {
                        warn!("An error occurred reporting progress: \"{error}\".");
                    }
                }

                Ok(())
            },
            None => Err(SynologyErrorStatus::NotLoggedIn)
        }
    }
}