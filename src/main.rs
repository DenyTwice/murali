/**! 
 This file contains the driver code that starts up the bot as well as all the implementations
 for it's commands.
*/

mod errors;
mod sheets;
mod misc;

use tracing::{event, span, Level};
use errors::Error;
use google_sheets4::api::ValueRange;
use poise::serenity_prelude as serenity;
use shuttle_poise::ShuttlePoise;
use shuttle_secrets::SecretStore;

/// Runtime storage of the bot.
struct Data {
    secret_store: SecretStore,
} 

/// Shorthand for poise::Context.
type Context<'a> = poise::Context<'a, Data, Error>;

/// `att` command adds the author's entry to the sheet.
#[poise::command(slash_command)]
async fn att(
    ctx: Context<'_>, 
    seat_number: Option<String>, 
    time_in: Option<String>, 
    time_out: Option<String>
    ) -> Result<(), Error> 
{

    let att_span = span!(Level::TRACE, "span: att");
    let _att_span = att_span.enter();

    event!(Level::TRACE, "Defering...");
    ctx.defer().await?; // Defering lets Discord know that the user interaction won't 
                        // be responded to immediately and holds the bot in a waiting
                        // state.

    let author = ctx.author().name.to_string();
    event!(Level::TRACE, author=author);

    event!(Level::DEBUG, "Fetching spreadsheet ID...");
    let spreadsheet_id = ctx.data().secret_store 
        .get("SPREADSHEET_ID")
        .expect("Spreadsheet ID must be set.");

    let template_id = String::from("0");

    event!(Level::DEBUG, "Getting member data...");
    let member_data = match misc::get_member_data(&author) {
        Ok(Some(data)) => data,
        Ok(None) => {
            let data_not_found_message = format!("No data was found for {}.", author);
            ctx.reply(data_not_found_message).await?;

            return Ok(());
        },
        Err(errors::GetRecordError::CSVError(_)) => {
            const CSV_ERROR_MESSAGE: &str = "Failed to read CSV records.";
            ctx.reply(CSV_ERROR_MESSAGE).await?;

            return Ok(());
        },
        Err(errors::GetRecordError::IOError(_)) => {
            const IO_ERROR_MESSAGE: &str = "Failed to open file MemberData.csv.";
            ctx.reply(IO_ERROR_MESSAGE).await?;

            return Ok(());
        },
    };
    
    event!(Level::DEBUG, "Building hub...");
    let hub = match sheets::build_hub().await {
        Ok(hub) => hub,
        Err(errors::BuildHubError::VarError(_)) => {
            const VAR_ERROR_MESSAGE: &str = "Failed to validate credentials";
            ctx.reply(VAR_ERROR_MESSAGE).await?;

            return Ok(())
        },
        Err(errors::BuildHubError::IOError(_)) => {
            const IO_ERROR_MESSAGE: &str = "Failed to read SA-C file."; 
            ctx.reply(IO_ERROR_MESSAGE).await?;

            return Ok(());
        },
    };

    event!(Level::DEBUG, "Computing next serial number...");
    let serial_num = match sheets::compute_next_serial_num(&hub, spreadsheet_id.as_str(), template_id.as_str()).await {
        Some(num) => num.try_into()?,
        None => {
            const COMPUTE_FAIL_MESSAGE: &str = "Failed to get serial number";
            ctx.reply(COMPUTE_FAIL_MESSAGE).await?;

            return Ok(());
        },
    };

    event!(Level::DEBUG, "Constructing input data...");
    let sheet_input = sheets::construct_input_data(serial_num, member_data, seat_number, time_in, time_out);
    
    event!(Level::DEBUG, "Inserting entry...");
    match sheets::insert_entry(spreadsheet_id.as_str(), hub, ValueRange::from(sheet_input)).await {
        Ok(()) => {
            ctx.reply("Okay.").await?;
            return Ok(());
        },
        Err(_) => {
            const INSERTION_ERROR_MESSAGE: &str = "Failed to insert field.";
            ctx.reply(INSERTION_ERROR_MESSAGE).await?;

            return Ok(());
        }
    }
}

#[shuttle_runtime::main]
async fn main(#[shuttle_secrets::Secrets] secret_store: SecretStore) -> ShuttlePoise<Data, Error>{

    let main_span = span!(Level::TRACE, "span: main");
    let _main_span = main_span.enter();

    event!(Level::TRACE, "Initializing FrameworkOptions...");
    let framework_options = poise::FrameworkOptions {
            commands: vec![att()],
            ..Default::default()
    };

    event!(Level::TRACE, "FrameworkOptions constructed.");
    event!(Level::DEBUG, "Fetching DISCORD_TOKEN from secret store...");
    let token = secret_store.get("DISCORD_TOKEN").expect("Discord Token must be set"); 

    event!(Level::TRACE, "Initializing Poise framework...");
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

    event!(Level::INFO, "Startup complete.");
    Ok(framework.into())
}
