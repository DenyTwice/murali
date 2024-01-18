mod errors;

use std::path::PathBuf;
use std::{env, env::VarError};
use std::fs::File;

// Third Party Crates
use google_sheets4::{self, Sheets};
use google_sheets4::api::ValueRange;
use poise::serenity_prelude as serenity;
use csv::{ReaderBuilder, StringRecord};
use serde_json::Value;

// Shuttle Deployment
use shuttle_secrets::SecretStore;
use shuttle_poise::ShuttlePoise;

// Bot storage
struct Data {
    secret_store: SecretStore,
} 

// Represents a row in the excel sheet
#[derive(Copy, Clone)]
struct Row<'a>{
    serial_number: usize,
    name: &'a str,
    roll_number: &'a str,
    seat_number: u32,
    time_in: &'a str,
    time_out: &'a str,
}

impl<'a> Row<'a> {
    fn pretty_print(&self) -> String {
        let message = format!("Appended data:\nSerial Number: {}\tName: {}\tRoll Number: {}\t
                              \t\tSeat Number: {}\tTime In: {}\t Time Out: {}\t", 
                              self.serial_number, self.name, self.roll_number, self.seat_number, self.time_in, self.time_out);
        return message;
    }
}

impl<'a> From<Row<'a>> for ValueRange {
    fn from(value: Row) -> Self {

        let values = Some(vec![vec![
                          Value::String(value.serial_number.to_string()),
                          Value::String(value.name.to_owned()),
                          Value::String(value.roll_number.to_owned()),
                          Value::String(value.seat_number.to_string()),
                          Value::String(value.time_in.to_owned()),
                          Value::String(value.time_out.to_owned())
        ]]);
        let range = format!("'{}'!1:6", chrono::Local::now().with_timezone(&chrono_tz::Asia::Kolkata).format("%e %b"));

        ValueRange { 
            major_dimension: Some(String::from("ROWS")), 
            range: Some(range),
            values 
        }
    }
}
// Central object to maintan state and access Google Sheets API
type SheetsHub = Sheets<hyper_rustls::HttpsConnector<yup_oauth2::hyper::client::HttpConnector>>;

// Custom Error type that points to generic that implements error::Error AND 
// Send, Sync which are thread-safety traits
type Error = Box<dyn std::error::Error + Send + Sync>;

// Context holds most of the runtime information such as the user which invoked a command 
// and has methods implemented on it that performs actions such as sending a message
type Context<'a> = poise::Context<'a, Data, Error>;

// Builds SheetsHub from SERVICE_ACCOUNT_CREDENTIALS through HTTPConnector
async fn build_hub(secret_store: &SecretStore) -> Result<Sheets<hyper_rustls::HttpsConnector<yup_oauth2::hyper::client::HttpConnector>>, errors::BuildHubError> {
    // !WARNING: Do not expose sa_credentials
    let sa_credentials_path = secret_store.get("SA_CREDENTIALS_PATH").expect("SA_CREDENTIALS_PATH must be set");

    // Auth using SA CREDENTIALS
    let mut path = PathBuf::new();
    path.push(env::current_dir()?);
    path.push(sa_credentials_path);
    let sa_credentials = yup_oauth2::read_service_account_key(path)
        .await?;
    let auth = yup_oauth2::ServiceAccountAuthenticator::builder(sa_credentials)
        .build()
        .await?;

    // Build google_sheets client through HttpConnector
    let hyper_client_builder = &google_sheets4::hyper::Client::builder();
    let http_connector_builder = hyper_rustls::HttpsConnectorBuilder::new();
    let http_connector_builder_options = http_connector_builder
        .with_native_roots()
        .https_or_http()
        .enable_http1()
        .build();

    Ok(Sheets::new(hyper_client_builder.build(http_connector_builder_options), auth))
}

async fn get_next_empty_row(secret_store: &SecretStore, range: &str, spreadsheet_id: &str) -> Option<usize> {

    // CAUTION: Should handle this error safely
    let hub = build_hub(secret_store).await.unwrap();
    let response = hub.spreadsheets().values_get(spreadsheet_id, range).doit().await.unwrap();
    let values = response.1;
    if let Some(rows) = values.values {
        return Some(rows.len());
    }
    None
}
async fn append_values_to_sheet(spreadsheet_id: &str, hub: SheetsHub, value_range: ValueRange) -> Result<(), ()>{

    // weird function, needs a struct and it's member as two different arguments
    // probably can refactor parent function to only take in the struct and then split it in here
    let range = value_range.range.clone().unwrap();
    let result = hub.spreadsheets().values_append(value_range, spreadsheet_id, range.as_str())
        .value_input_option("USER_ENTERED")
        .doit()
        .await;

    match result {
        Ok(_) => return Ok(()),
        Err(e) => {
            eprintln!("Error: {:?}", e);
            return Err(());
        }
    }
}

// Uses predefined CSV to find data about member from their discord username (passed in as key) 
// Returns:
// Ok<Some> if member data found, Ok<None> otherwise
// Err() if failed in execution
fn get_member_record(key: &str) -> Result<Option<StringRecord>, errors::GetRecordError> {
    let file = File::open("./MemberData.csv")?;
    let mut rdr = ReaderBuilder::new().from_reader(file);
    let csv_iter = rdr.records();

    for result in csv_iter {
        if let Ok(record) = result { 
            let user_name = record.get(0).expect("Members data must be set");
            if user_name == key {
                return Ok(Some(record));
            }
        } else if let Err(e) = result {
            println!("Could not read record");
            return Err(errors::GetRecordError::CSVError(e));
        }
    };

    Ok(None)
}

#[poise::command(slash_command)]
async fn att(ctx: Context<'_>, seat_number: u32, mut time_in: Option<String>, mut time_out: Option<String>) -> Result<(), Error> {
    let spreadsheet_id = ctx.data().secret_store.get("SPREADSHEET_ID").expect("SPREADSHEET");

    let author = ctx.author().name.to_string();
    
    // Maybe extract this into another function for readability
    match get_member_record(author.as_str()) {
        Ok(record_option) => {
            if let Some(record) = record_option {

                if let None = time_in {
                    time_in = Some(chrono::Local::now().with_timezone(&chrono_tz::Asia::Kolkata).format("%H:%M").to_string());
                };

                if let None = time_out {
                    if record.get(3).unwrap() == "M" {
                        time_out = Some(String::from("23:00"));
                    } else {
                        time_out = Some(String::from("21:00"));
                    }
                }
                // God knows why temporary values can't be dropped
                let time_in_unwrapped = time_in.unwrap();
                let time_out_unwrapped = time_out.unwrap();
                // Lots of unwrap() here since they are hardcoded records and
                // shouldn't fail unless the OS does

                let range = format!("'{}'!1:6", chrono::Local::now().with_timezone(&chrono_tz::Asia::Kolkata).format("%e %b"));
                let serial_num = get_next_empty_row(&ctx.data().secret_store, range.as_str(), spreadsheet_id.as_str()).await.unwrap();
                let row: Row = Row {
                    serial_number: serial_num, // make dynamic
                    name: record.get(1).unwrap(),
                    roll_number: record.get(2).unwrap(),
                    seat_number,
                    time_in: time_in_unwrapped.as_str(),
                    time_out: time_out_unwrapped.as_str(),
                };

                // CAUTION: Should handle this error safely
                let hub = build_hub(&ctx.data().secret_store).await.unwrap();
                match append_values_to_sheet(spreadsheet_id.as_str(), hub, ValueRange::from(row)).await {
                    // Send appended data and/or log errors
                    Ok(_) => {
                        print!("yes");
                        let message = row.pretty_print();
                        ctx.reply(message).await?;
                        return Ok(());
                    }
                    Err(_) => ctx.reply(String::from("No")).await?,
                };
            } else {
                ctx.reply(format!("Could not find {}'s data", author)).await?;
            }
        },
        // Log error e
        Err(e) => {
            ctx.reply(String::from("Ran into error while trying to find user data")).await?;
            return Ok(());
        }
    };

    Ok(())
}

#[shuttle_runtime::main]
async fn main(#[shuttle_secrets::Secrets] secret_store: SecretStore) -> ShuttlePoise<Data, Error>{
    let framework_options = poise::FrameworkOptions {
            commands: vec![att()],
            ..Default::default()
    };

    // !WARNING: Do NOT expose Discord Bot Token
    let token = secret_store.get("DISCORD_TOKEN").expect("Discord Token must be set"); 
    let framework = poise::Framework::builder()
        .options(framework_options)
        .token(token)
        .intents(serenity::GatewayIntents::non_privileged())
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data { secret_store })
            })
        })
        .build()
        .await
        .map_err(shuttle_runtime::CustomError::new)?;

    // framework.run().await.unwrap();
    Ok(framework.into())
}

