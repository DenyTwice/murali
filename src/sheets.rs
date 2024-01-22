use crate::errors;
use crate::misc;
use crate::misc::MemberData; 

use std::path::PathBuf;
use std::env;
use shuttle_secrets::SecretStore;
use google_sheets4::api::ValueRange;
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
    pub seat_number: u32,
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
pub async fn build_hub(secret_store: &SecretStore) -> Result<SheetsHub, errors::BuildHubError> 
{
    let sa_credentials_path = secret_store.get("SA_CREDENTIALS_PATH")
        .expect("SA_CREDENTIALS_PATH must be set");

    let mut path = PathBuf::new();
    path.push(env::current_dir()?);
    path.push(sa_credentials_path);

    let sa_credentials = yup_oauth2::read_service_account_key(path)
        .await?;

    let auth = yup_oauth2::ServiceAccountAuthenticator::builder(sa_credentials)
        .build()
        .await?;

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
pub async fn compute_next_serial_num(hub: &SheetsHub, spreadsheet_id: &str) -> Option<u32> 
{
    let date = format!("{}", chrono::Local::now()
                       .with_timezone(&chrono_tz::Asia::Kolkata)
                       .format("%e %b"));

    let range = format!("{}!1:50", date.trim());
    let response = hub.spreadsheets()
        .values_get(spreadsheet_id, range.as_str())
        .doit()
        .await
        .unwrap();

    let values = response.1;
    if let Some(rows) = values.values {
        return Some(rows.len().try_into().unwrap());
    }

    None
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
    seat_number: u32, 
    time_in: Option<String>, 
    time_out: Option<String>
    ) -> Row 
{

    let (time_in, time_out) = misc::set_time(time_in, time_out, member_data.gender);

    Row {
        serial_number,
        name: member_data.name,
        roll_number: member_data.roll_number,
        seat_number,
        time_in,
        time_out
    }
}
