/*!
This file cuontains custom error enums and corresponding conversion 
functions. Eventually, I hope to replace this with the "anyhow" crate
or with Eyre. Seems too tedious to do this for every scenario anyway.
*/

use std::io::Error as IOError;
use std::env::VarError;

/** 
 * Custom Error type that points to generic that implements error::Error and 
 * Send, Sync which are thread-safety traits.
 */
pub type Error = Box<dyn std::error::Error + Send + Sync>;

/** 
 * Errors for file::get_member_record.
 *
 * IOError occurs when failing to open the CSV file for reading.
 * CSVError occurs when Rust fails to read a record in the CSV file.
 */
pub enum GetRecordError {
    IOError(IOError),
    CSVError(csv::Error),
}

/** 
 * Errors for sheets::build_hub.
 *
 * VarError occurs when failing to read SA_CREDENTIALS from env.
 * IOError occurs when yup_oauth2 fails to read or validate SA_CREDENTIALS.
 */
pub enum BuildHubError {
    VarError(VarError),
    IOError(IOError),
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
