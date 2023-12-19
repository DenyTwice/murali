/**
This file cuontains custom error enums and corresponding conversion 
functions. Eventually, I hope to replace this with the "anyhow" crate
or with Eyre. Seems too tedious to do this for every scenario anyway.
*/


use std::io::Error as IOError;
use std::env::VarError;

// Errors for main::get_member_record
pub enum GetRecordError {
    // IOError occurs when failing to open the CSV file for reading
    IOError(IOError),
    // CSVError occurs when Rust fails to read a record (could be any record,
    // not just the key) in the CSV file
    CSVError(csv::Error),
}

impl From<IOError> for GetRecordError {
    fn from(value: IOError) -> Self {
        GetRecordError::IOError(value)
    }
}

impl From<csv::Error> for GetRecordError {
    fn from(value: csv::Error) -> Self {
        GetRecordError::CSVError(value)
    }
}

// Errors for sheets::build_hub
#[derive(Debug)]
pub enum BuildHubError {
    // VarError occurs when failing to read SA_CREDENTIALS from env
    VarError(VarError),
    // IOError occurs when yup_oauth2 fails to read or validate SA_CREDENTIALS
    IOError(IOError),
}

impl From<VarError> for BuildHubError {
    fn from(value: VarError) -> Self {
        BuildHubError::VarError(value)
    }
}

impl From<IOError> for BuildHubError {
    fn from(value: IOError) -> Self {
        BuildHubError::IOError(value)
    }
}
