use std::collections::HashMap;

use num_derive::FromPrimitive;
use reqwest::StatusCode;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
#[derive(FromPrimitive)]
pub enum SynologyStatusCode {
    #[error("Unknown error")]
    UnknownError = 100,
    #[error("No parameter of API, method or version")]
    NoParameterOfApiMethodOrVersion = 101,
    #[error("The requested API does not exist")]
    RequestedApiDoesNotExist = 102,
    #[error("The requested method does not exist")]
    RequestedMethodDoesNotExist = 103,
    #[error("The requested version does not support the functionality")]
    RequestedVersionDoesNotSupportFunctionality = 104,
    #[error("The logged in session does not have permission")]
    LoggedInSessionDoesNotHavePermission = 105,
    #[error("Session timeout")]
    SessionTimeout = 106,
    #[error("Session interrupted by duplicate login")]
    SessionInterruptedByDuplicateLogin = 107,
    #[error("SID not found")]
    SidNotFound = 119,
    #[error("Invalid parameter of file operation")]
    InvalidParameterOfFileOperation = 400,
    #[error("Unknown error of file operation")]
    UnknownErrorOfFileOperation = 401,
    #[error("System is too busy")]
    SystemIsTooBusy = 402,
    #[error("Invalid user does this file operation")]
    InvalidUserDoesThisFileOperation = 403,
    #[error("Invalid group does this file operation")]
    InvalidGroupDoesThisFileOperation = 404,
    #[error("Invalid user and group does this file operation")]
    InvalidUserAndGroupDoesThisFileOperation = 405,
    #[error("Can't get user/group information from the account server")]
    CantGetUserGroupInformationFromTheAccountServer = 406,
    #[error("Operation not permitted")]
    OperationNotPermitted = 407,
    #[error("No such file or directory")]
    NoSuchFileOrDirectory = 408,
    #[error("Non-supported file system")]
    NonSupportedFileSystem = 409,
    #[error("Failed to connect internet-based file system (e.g., CIFS)")]
    FailedToConnectInternetBasedFileSystem = 410,
    #[error("Read-only file system")]
    ReadOnlyFileSystem = 411,
    #[error("Filename too long in the non-encrypted file system")]
    FilenameTooLongInTheNonEncryptedFileSystem = 412,
    #[error("Filename too long in the encrypted file system")]
    FilenameTooLongInTheEncryptedFileSystem = 413,
    #[error("File already exists")]
    FileAlreadyExists = 414,
    #[error("Disk quota exceeded")]
    DiskQuotaExceeded = 415,
    #[error("No space left on device")]
    NoSpaceLeftOnDevice = 416,
    #[error("Input/output error")]
    InputOutputError = 417,
    #[error("Illegal name or path")]
    IllegalNameOrPath = 418,
    #[error("Illegal file name")]
    IllegalFileName = 419,
    #[error("Illegal file name on FAT file system")]
    IllegalFileNameOnFatFileSystem = 420,
    #[error("Device or resource busy")]
    DeviceOrResourceBusy = 421,
    #[error("No such task of the file operation")]
    NoSuchTaskOfTheFileOperation = 599
}

#[derive(Error, Debug)]
pub enum SynologyErrorStatus {
    #[error(transparent)]
    ServerError(#[from] SynologyStatusCode),
    #[error("HTTP error occurred.")]
    HttpError(StatusCode),
    #[error(transparent)]
    ReqwestError(#[from] reqwest::Error),
    #[error(transparent)]
    SerdeError(#[from] serde_json::Error),
    #[error("TOTP required but not provided")]
    NoTotp,
    #[error("No user logged in")]
    NotLoggedIn,
    #[error("An unknown error occurred.")]
    UnknownError
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde[rename_all = "snake_case"]]
pub struct SynologyResult<TData, TErrors> {
    pub success: bool,
    pub data: Option<TData>,
    pub error: Option<SynologyError<TErrors>>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde[rename_all = "snake_case"]]
pub struct SynologyError<TErrors> {
    pub code: u32,
    pub errors: Option<TErrors>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde[rename_all = "snake_case"]]
pub struct LoginResult {
    pub sid: String
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde[rename_all = "snake_case"]]
pub struct LoginError {
    pub token: String,
    pub types: Vec<HashMap<String, String>>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde[rename_all = "snake_case"]]
pub struct CreateFolderResult {
    folders: Vec<FolderModel>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde[rename_all = "snake_case"]]
pub struct FolderModel {
    pub isdir: bool,
    pub name: String,
    pub path: String
}