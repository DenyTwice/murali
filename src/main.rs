use shuttle_poise::ShuttlePoise;
use shuttle_secrets::SecretStore;

use std::path::PathBuf;
use std::{env, env::VarError, io::Error as IOError, process::exit};

use google_sheets4::{self, Sheets};
use poise::serenity_prelude as serenity;

// Bot storage
struct Data {} 

// Custom Error type that points to generic that implements error::Error AND 
// Send, Sync which are thread-safety traits
type Error = Box<dyn std::error::Error + Send + Sync>;

// Context holds most of the runtime information such as the user which invoked a command 
// and has methods implemented on it that performs actions such as sending a message
type Context<'a> = poise::Context<'a, Data, Error>;

enum BuildHubError {
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

// build_sheets_api builds the "sheets_hub" for google_sheets4, which is the central object to maintain
// state and access all "activities" 
async fn build_hub(secret_store: &SecretStore) -> Result<Sheets<hyper_rustls::HttpsConnector<yup_oauth2::hyper::client::HttpConnector>>, BuildHubError> {
    // !WARNING: Do not expose sa_credentials
    let sa_credentials_path = secret_store.get("SA_CREDENTIALS_PATH").expect("SA_CREDENTIALS_PATH must be set");

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

// append_values_to_sheet adds values to the specified excel sheet
// hub: Sheets hub 
// range: Sheet ID + Table Constraints,
// values: Struct that contains JSON::values that are to be appended to the sheet
async fn append_values_to_sheet(hub: Sheets<hyper_rustls::HttpsConnector<yup_oauth2::hyper::client::HttpConnector>>, range: String, values: google_sheets4::api::ValueRange) {

    let spreadsheet_id = std::env::var("SPREADSHEET_ID").expect("Spreadsheet ID must be set");
    let result = hub.spreadsheets().values_append(values, spreadsheet_id.as_str(), range.as_str())
        .value_input_option("USER_ENTERED")
        .doit()
        .await;

    match result {
        Ok(resp) => println!("Append Response: {:?}", resp),
        Err(e) => eprintln!("Error: {:?}", e),
    }
}

#[poise::command(slash_command)]
async fn att(ctx: Context<'_>, message: Option<u32>) -> Result<(), Error> {
    ctx.say(String::from("WIP")).await?;
    Ok(())
}

#[shuttle_runtime::main]
async fn main(#[shuttle_secrets::Secrets] secret_store: SecretStore) -> ShuttlePoise<Data, Error>{

    let sheets_hub = match build_hub(&secret_store).await {
        Ok(hub) => hub,
        Err(err) => match err {
            BuildHubError::VarError(var_err) => {
                println!("ERROR: build_hub failed. Could not read SA_CREDENTIALS from .env");
                eprintln!("{var_err}");
                exit(1);
            },
            BuildHubError::IOError(io_err) => {
                println!("ERROR: build_hub failed. yup_oauth2 to read or validate SA_CREDENTIALS.");
                eprintln!("{io_err}");
                exit(1);
            },
        }
    };

    let framework_options = poise::FrameworkOptions {
            commands: vec![att()],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some(String::from(".")),
                ..Default::default()
            },
            ..Default::default()
    };
    // let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN"); // !WARNING: Do NOT expose Discord Bot Token
    let token = secret_store.get("DISCORD_TOKEN").expect("Discord Token must be set"); 
    let framework = poise::Framework::builder()
        .options(framework_options)
        .token(token)
        .intents(serenity::GatewayIntents::non_privileged())
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        })
        .build()
        .await
        .map_err(shuttle_runtime::CustomError::new)?;

    // framework.run().await.unwrap();
    Ok(framework.into())
}

