/*!
This module contains code that uses the google_sheets4 API to manipulate and update sheets as well as
any revelant helper code.
*/

use crate::errors;
use crate::misc;
use crate::misc::MemberData; 

use tracing::{span, event, Level};
use std::path::PathBuf;
use std::env;
use google_sheets4::api::{ValueRange, DuplicateSheetRequest, BatchUpdateSpreadsheetRequest, Request};
use google_sheets4::{self, Sheets};
use serde_json::Value;

/// Central object to access Google Sheets API
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
  Builds `SheetsHub` from `SERVICE_ACCOUNT_CREDENTIALS` through `HTTPConnector`.

  Returns `Ok<SheetHub>` or `Err<BuildHubError>` if it fails in execution.
*/
pub async fn build_hub() -> Result<SheetsHub, errors::BuildHubError> 
{
    let build_hub_span = span!(Level::TRACE, "span: build_hub");
    let _build_hub_span = build_hub_span.enter();

    let sa_credentials_path = PathBuf::from("secrets/sa_credentials.json");

    event!(Level::DEBUG, "Creating path to read SA Credentials...");
    let mut path = PathBuf::new();
    path.push(env::current_dir()?);
    path.push(sa_credentials_path);


    event!(Level::DEBUG, "Reading service account key...");
    let sa_credentials = yup_oauth2::read_service_account_key(path)
        .await?;

    event!(Level::DEBUG, "Authenticating with SA-C...");
    let auth = yup_oauth2::ServiceAccountAuthenticator::builder(sa_credentials)
        .build()
        .await?;

    event!(Level::DEBUG, "Building http_connector...");
    let hyper_client_builder = &google_sheets4::hyper::Client::builder();
    let http_connector_builder = hyper_rustls::HttpsConnectorBuilder::new();
    let http_connector_builder_options = http_connector_builder
        .with_native_roots()
        .https_or_http()
        .enable_http1()
        .build();

    event!(Level::TRACE, "Hub succesfully built. Returning...");
    Ok(Sheets::new(hyper_client_builder.build(http_connector_builder_options), auth))
}

/** 
  Gets the length of the array of all fields with data in the attendance sheet.
 
  Returns number of rows that have data or None (When?).
 */ 
pub async fn compute_next_serial_num(hub: &SheetsHub, spreadsheet_id: &str, template_id: &str) -> Option<u32> {
    let computer_next_serial_num_span = span!(Level::TRACE, "span: compute_next_serial_num");
    let _computer_next_serial_num_span = computer_next_serial_num_span.enter();

    let date = format!("{}", chrono::Local::now()
                       .with_timezone(&chrono_tz::Asia::Kolkata)
                       .format("%e %b"));
    let trimmed_date = date.trim();
    event!(Level::DEBUG, "Date set to {date}");

    let range = format!("{}!1:50", trimmed_date);
    event!(Level::TRACE, "Getting values from spreadsheet...");
    let response = hub.spreadsheets()
        .values_get(spreadsheet_id, range.as_str())
        .doit()
        .await;

    match response {
        Ok(response) => {
            event!(Level::TRACE, "Succesful response.");
            let values = response.1;
            if let Some(rows) = values.values {
                return Some(rows.len().try_into().unwrap());
            }
        }
        Err(google_sheets4::Error::BadRequest(status)) => {
            // Check if the error message contains "Unable to parse range"
            // If it does, then assume the sheet for the current date does not exist.
            event!(Level::DEBUG, "Bad request.");
            let error_message = format!("{:?}", status);
            if error_message.contains("Unable to parse range") {
                event!(Level::DEBUG, "Error: {error_message}.");
                event!(Level::DEBUG, "Creating duplicate sheet...");
                let _ = duplicate_sheet(hub, spreadsheet_id, template_id, trimmed_date).await.ok()?;
                return Some(1)
            }
        },
        Err(e) => {
            event!(Level::DEBUG, "Error while trying to match response: {e}");
        }
    }
    None
}


/**
  Duplicates the template sheet.

  Returns standard `Result`.
 */
pub async fn duplicate_sheet(hub: &SheetsHub, spreadsheet_id: &str, template_id: &str, new_sheet_name: &str) -> Result<(), ()> {
    let duplicate_sheet_span = span!(Level::TRACE, "span: duplicate_sheet");
    let _duplicate_sheet_span = duplicate_sheet_span.enter();

    event!(Level::TRACE, "Creating DuplicateSheetRequest...");
    let request = DuplicateSheetRequest {
        insert_sheet_index: Some(1),
        new_sheet_name: Some(new_sheet_name.to_string()),
        source_sheet_id: Some(template_id.parse().unwrap()),
        ..Default::default()
    };
    
    event!(Level::TRACE, "Creating BatchUpdateSpreadsheetRequest...");
    let batch_update_request = BatchUpdateSpreadsheetRequest {
        requests: Some(vec![Request {
            duplicate_sheet: Some(request),
            ..Default::default()
        }]),
        ..Default::default()
    };

    event!(Level::TRACE, "Updating by batch...");
    let result = hub.spreadsheets().batch_update(batch_update_request, spreadsheet_id)
        .doit()
        .await;

    match result {
        Ok(_) => {
            event!(Level::DEBUG, "Batch update successful.");
            Ok(())
        },
        Err(e) => {
            event!(Level::DEBUG, "Batch update failed. Error: {e}");
            Err(())
        },
    }
}

/**
  Appends the data within `ValueRange` to the excel sheet.

  Returns standard `Result`.
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

/// Prepares to append into the excel sheet by grouping necessary data into a neat struct. 
pub fn construct_input_data(
    serial_number: u32, 
    member_data: MemberData, 
    mut seat_number: Option<String>, 
    time_in: Option<String>, 
    time_out: Option<String>
    ) -> Row 
{

    let construct_input_data_span = span!(Level::TRACE, "span: construct_input_data");
    let _construct_input_data_span = construct_input_data_span.enter();

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
