use crate::errors;
use crate::misc;
use crate::misc::MemberData; 

use std::path::PathBuf;
use std::env;
use shuttle_secrets::SecretStore;
use google_sheets4::api::{ValueRange, DuplicateSheetRequest, BatchUpdateSpreadsheetRequest, Request};
use google_sheets4::{self, Sheets};
use serde_json::Value;

/// Central object to maintan state and access Google Sheets API
pub type SheetsHub = Sheets<hyper_rustls::HttpsConnector<yup_oauth2::hyper::client::HttpConnector>>;

/// Represents a row/field in the excel sheet
#[derive(Clone)]
pub struct Row {
    pub serial_number: u32,
    pub name: String,
    pub roll_number: String,
    pub seat_number: String,
    pub time_in: String,
    pub time_out: String,
}

impl Row {
    pub fn pretty_print(&self) -> String 
    {
        let message = format!("Appended data:\n{}. {}\t{}\t{}\t{}\t{}", 
                              self.serial_number, self.name, self.roll_number, self.seat_number, self.time_in, self.time_out);
        return message;
    }
}

/// ValueRange is the accepted type for inserting data into the sheet.
impl From<Row> for ValueRange {
    fn from(value: Row) -> Self 
    {
        let values = Some(vec![vec![
                          Value::String(value.serial_number.to_string()),
                          Value::String(value.name),
                          Value::String(value.roll_number),
                          Value::String(value.seat_number.to_string()),
                          Value::String(value.time_in),
                          Value::String(value.time_out)
        ]]);

        let date = format!("{}", chrono::Local::now()
                           .with_timezone(&chrono_tz::Asia::Kolkata)
                           .format("%e %b")
                          );

        let range = format!("{}!1:50", date.trim());

        ValueRange { 
            major_dimension: Some(String::from("ROWS")), 
            range: Some(range),
            values 
        }
    }
}

/**
 * @brief   Builds SheetsHub from SERVICE_ACCOUNT_CREDENTIALS through HTTPConnector
 *
 * @return  Ok<SheetHub>
 *          Err<BuildHubError>, if it fails in execution
 */
pub async fn build_hub() -> Result<SheetsHub, errors::BuildHubError> 
{
    let sa_credentials_path = PathBuf::from("secrets/sa_credentials.json");

    let mut path = PathBuf::new();
    path.push(sa_credentials_path);

    let sa_credentials = yup_oauth2::read_service_account_key(path)
        .await?;

    print!("it did read");
    let auth = yup_oauth2::ServiceAccountAuthenticator::builder(sa_credentials)
        .build()
        .await?;

    print!("it did read and auth");
    let hyper_client_builder = &google_sheets4::hyper::Client::builder();
    let http_connector_builder = hyper_rustls::HttpsConnectorBuilder::new();
    let http_connector_builder_options = http_connector_builder
        .with_native_roots()
        .https_or_http()
        .enable_http1()
        .build();

    Ok(Sheets::new(hyper_client_builder.build(http_connector_builder_options), auth))
}

/** 
 * @brief   Gets the length of the array of fields in the attendance sheet returned from the API.
 *
 * @return  Number of rows that have data.
 */ 
pub async fn compute_next_serial_num(hub: &SheetsHub, spreadsheet_id: &str, template_id: &str) -> Option<u32> {
    let date = format!("{}", chrono::Local::now()
                       .with_timezone(&chrono_tz::Asia::Kolkata)
                       .format("%e %b"));
    let trimmed_date = date.trim();

    let range = format!("{}!1:50", trimmed_date);
    let response = hub.spreadsheets()
        .values_get(spreadsheet_id, range.as_str())
        .doit()
        .await;

    match response {
        Ok(response) => {
            let values = response.1;
            if let Some(rows) = values.values {
                return Some(rows.len().try_into().unwrap());
            }
        }
        Err(google_sheets4::Error::BadRequest(status)) => {
            // Check if the error message contains "Unable to parse range"
            // If it does, then the sheet for the current date does not exist.
            let error_message = format!("{:?}", status);
            if error_message.contains("Unable to parse range") {
                let _ = duplicate_sheet(hub, spreadsheet_id, template_id, trimmed_date).await.ok()?;
                return Some(1)
            }
        },
        Err(e) => {
            println!("Error: {:?}", e);
        }
    }
    None
}


/**
 * @brief   Duplicates the template sheet to a new sheet with the given name.
 *
 * @return  Ok(())
 *          Err(())
 */
pub async fn duplicate_sheet(hub: &SheetsHub, spreadsheet_id: &str, template_id: &str, new_sheet_name: &str) -> Result<(), ()> {
    let request = DuplicateSheetRequest {
        insert_sheet_index: Some(1),
        new_sheet_name: Some(new_sheet_name.to_string()),
        source_sheet_id: Some(template_id.parse().unwrap()),
        ..Default::default()
    };
    
    let batch_update_request = BatchUpdateSpreadsheetRequest {
        requests: Some(vec![Request {
            duplicate_sheet: Some(request),
            ..Default::default()
        }]),
        ..Default::default()
    };

    let result = hub.spreadsheets().batch_update(batch_update_request, spreadsheet_id)
        .doit()
        .await;

    match result {
        Ok(_) => Ok(()),
        Err(_) => Err(()),
    }
}

/**
 * @brief   Appends the data within ValueRange to the excel sheet.
 *
 * @return  Ok(())
 *          Err(())
 */
pub async fn insert_entry(
    spreadsheet_id: &str, 
    hub: SheetsHub, 
    value_range: ValueRange
    ) -> Result<(), ()> 
{
    let range = value_range.range
        .clone()
        .unwrap();

    let result = hub.spreadsheets()
        .values_append(value_range, spreadsheet_id, range.as_str())
        .value_input_option("USER_ENTERED")
        .doit()
        .await;

    match result {
        Ok(_) => return Ok(()),
        Err(_) => return Err(()),
    }
}

pub fn construct_input_data(
    serial_number: u32, 
    member_data: MemberData, 
    mut seat_number: Option<String>, 
    time_in: Option<String>, 
    time_out: Option<String>
    ) -> Row 
{

    let (time_in, time_out) = misc::set_time(time_in, time_out, member_data.gender);

    if let None = seat_number {
        seat_number = Some(String::from(""));
    }

    Row {
        serial_number,
        name: member_data.name,
        roll_number: member_data.roll_number,
        seat_number: seat_number.unwrap(),
        time_in,
        time_out
    }
}
