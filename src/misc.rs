//! This file contains helper functions intended to modularize the code and increase readability in other modules. 

use crate::errors;

use tracing::{span, event, Level};
use std::fs::File;
use csv::ReaderBuilder;

/// Exists primarily to make function headers contain less arguments.
pub struct MemberData {
    pub name: String,
    pub gender: String,
    pub roll_number: String
}

/**
  Searches predefined CSV to find real name, roll number and gender of a person with 
  their discord username as the key. Returns `Ok<Some>` if member data found, 
  `Ok<None>` if otherwise. Returns `Err()` if failed in execution.
 */
pub fn get_member_data(key: &str) -> Result<Option<MemberData>, errors::GetRecordError> 
{
    let get_member_data_span = span!(Level::TRACE, "span: get_member_data");
    let _get_meber_data_span = get_member_data_span.enter();

    const RECORD_GET_EXPECT_MESSAGE: &str = "Members data must be set";

    event!(Level::DEBUG, "Opening MemberData.csv");
    let file = File::open("secrets/MemberData.csv")?;
    let mut rdr = ReaderBuilder::new()
        .has_headers(false)
        .from_reader(file);

    let csv_iter = rdr.records();
    event!(Level::TRACE, "Iterating through records...");
    for item in csv_iter {
        if let Ok(record) = item { 
            event!(Level::TRACE, "Record found: {:?}.", record);
            let user_name = record.get(0).expect(RECORD_GET_EXPECT_MESSAGE);

            if user_name == key {
            event!(Level::TRACE, "Key found: {:?}.", record);
                return Ok(Some(
                        MemberData {
                            name: record.get(1).expect(RECORD_GET_EXPECT_MESSAGE).to_owned(),
                            gender: record.get(3).expect(RECORD_GET_EXPECT_MESSAGE).to_owned(),
                            roll_number: record.get(2).expect(RECORD_GET_EXPECT_MESSAGE).to_owned()
                        }
                        ));
            }

        } else if let Err(e) = item {
            event!(Level::TRACE, "Could not open record.");
            return Err(crate::errors::GetRecordError::CSVError(e));
        }
    };

    event!(Level::TRACE, "No records matching key found.");
    Ok(None)
}

/// Safely unwrap `time_in` and `time_out` by setting them default values if required.
pub fn set_time(time_in_opt: Option<String>, time_out_opt: Option<String>, gender: String) -> (String, String) {
    let set_time_span = span!(Level::TRACE, "span: set_time");
    let _set_time_span = set_time_span.enter();

    let mut time_in = String::new();
    let mut time_out = String::new();

    if let None = time_in_opt {
        event!(Level::TRACE, "Setting default value for time_in...");
        time_in.push_str("17:30");
    } else {
        event!(Level::TRACE, "Unwrapping time_in...");
        time_in.push_str(time_in_opt.unwrap().as_str());
    }

    if let None = time_out_opt {
        event!(Level::TRACE, "Setting default value for time_in...");
        if gender == "M" {
            time_out.push_str("22:00");
        } else {
            time_out.push_str("21:00");
        }
    } else {
        event!(Level::TRACE, "Unwrapping time_out...");
        time_out.push_str(time_out_opt.unwrap().as_str());
    }

    (time_in, time_out)
}
