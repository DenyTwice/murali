use crate::errors;
use std::fs::File;
use csv::{ReaderBuilder, StringRecord};

// Uses predefined CSV to find data about member from their discord username which is passed in as key
// Returns:
// Ok<Some> if member data found, Ok<None> otherwise
// Err() if failed in execution
pub fn get_member_record(key: &str) -> Result<Option<StringRecord>, errors::GetRecordError> {
    let file = File::open("MemberData.csv")?;
    // Log success in opening file
    let mut rdr = ReaderBuilder::new()
        .has_headers(false)
        .from_reader(file);
    let csv_iter = rdr.records();

    for item in csv_iter {
        if let Ok(record) = item { 
            // Failing this is a item of incorrectly set CSV file.
            let user_name = record.get(0).expect("Members data must be set");
            // Log failing to get record
            if user_name == key {
                // Log success in finding record
                return Ok(Some(record));
            }
        } else if let Err(e) = item {
            // Replace with logger
            println!("Could not read record");
            return Err(crate::errors::GetRecordError::CSVError(e));
        }
    };

    Ok(None)
}
