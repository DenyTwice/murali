use std::io::Error as IOError;
use std::env::VarError;

pub enum GetRecordError {
    IOError(IOError),
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

