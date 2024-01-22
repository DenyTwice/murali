mod errors;
mod sheets;
mod misc;

use errors::Error;
use google_sheets4::api::ValueRange;
use poise::serenity_prelude as serenity;
use shuttle_poise::ShuttlePoise;
use shuttle_secrets::SecretStore;

struct Data {
    secret_store: SecretStore,
} 

/** 
 * Context holds most of the runtime information such as the user who invoked a command,
 * and has methods implemented on it that performs actions such as sending a message. 
 */
type Context<'a> = poise::Context<'a, Data, Error>;

/// Command to add an entry for the author to the attendance sheet.
#[poise::command(slash_command)]
async fn att(
    ctx: Context<'_>, 
    seat_number: Option<String>, 
    time_in: Option<String>, 
    time_out: Option<String>
    ) -> Result<(), Error> 
{

    ctx.defer().await?;

    let author = ctx.author().name.to_string();
    let spreadsheet_id = ctx.data().secret_store 
        .get("SPREADSHEET_ID") // ID of the attendance sheet. 
        .expect("Spreadsheet ID must be set."); 

    // Gets name, gender and roll number
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
    
    let hub = match sheets::build_hub(&ctx.data().secret_store).await {
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

    let serial_num = match sheets::compute_next_serial_num(&hub, spreadsheet_id.as_str()).await {
        Some(num) => num.try_into()?,
        None => {
            const COMPUTE_FAIL_MESSAGE: &str = "Failed to get serial number";
            ctx.reply(COMPUTE_FAIL_MESSAGE).await?;

            return Ok(());
        },
    };

    let sheet_input = sheets::construct_input_data(serial_num, member_data, seat_number, time_in, time_out);
    
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
    let framework_options = poise::FrameworkOptions {
            commands: vec![att()],
            ..Default::default()
    };

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

    Ok(framework.into())
}
