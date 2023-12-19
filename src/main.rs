mod errors;
mod sheets;

use std::fs::File;

// Third Party Crates
use google_sheets4::api::ValueRange;
use poise::serenity_prelude as serenity;
use csv::{ReaderBuilder, StringRecord};

// Shuttle Deployment
use shuttle_secrets::SecretStore;
use shuttle_poise::ShuttlePoise;

// Bot storage
struct Data {
    secret_store: SecretStore,
} 

// Custom Error type that points to generic that implements error::Error AND 
// Send, Sync which are thread-safety traits
type Error = Box<dyn std::error::Error + Send + Sync>;

// Context holds most of the runtime information such as the user which invoked a command 
// and has methods implemented on it that performs actions such as sending a message
type Context<'a> = poise::Context<'a, Data, Error>;

// Uses predefined CSV to find data about member from their discord username which is passed in as key
// Returns:
// Ok<Some> if member data found, Ok<None> otherwise
// Err() if failed in execution
fn get_member_record(key: &str) -> Result<Option<StringRecord>, errors::GetRecordError> {
    let file = File::open("MemberData.csv")?;
    // Log success in opening file
    let mut rdr = ReaderBuilder::new().from_reader(file);
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
            return Err(errors::GetRecordError::CSVError(e));
        }
    };

    Ok(None)
}

#[poise::command(slash_command)]
async fn att(ctx: Context<'_>, seat_number: u32, mut time_in: Option<String>, mut time_out: Option<String>) -> Result<(), Error> {
    let spreadsheet_id = ctx.data().secret_store.get("SPREADSHEET_ID").expect("SPREADSHEET");
    // Log sucess
    let author = ctx.author().name.to_string();

    // Getting member data and appending to sheet might cause interaction
    // to timeout so defer holds the interaction alive long enough.
    ctx.defer().await?;
    // Log await failure
    
    // Maybe extract this into another function for readability
    match get_member_record(author.as_str()) {
        Ok(record_option) => {

            if let Some(record) = record_option {

                // If time_in is not specified, set it to the current time
                if let None = time_in {
                    time_in = Some(chrono::Local::now().with_timezone(&chrono_tz::Asia::Kolkata).format("%H:%M").to_string());
                    // Log setting time_in or time_in
                };

                // If time_out is not specified, set it to 22:45 or 21:00 depending on whether
                // the author is male or female
                if let None = time_out {
                    // Failing to get(3) is only possible if the CSV file is not set correctly.
                    if record.get(3).unwrap() == "M" {
                        time_out = Some(String::from("23:00"));
                    } else {
                        time_out = Some(String::from("21:00"));
                    }
                    // Log time_out
                }

                // God knows why temporary values can't be dropped
                // Log attempt to unwrap
                let time_in_unwrapped = time_in.unwrap();
                let time_out_unwrapped = time_out.unwrap();
                // Log sucess
 
                // The range determines which sheet and "where" the table is
                // Required format: '{Current date}'!1:6
                // 1:6 indicates that the table starts from column one and ends at column six
                let range = format!("'{}'!1:6", chrono::Local::now().with_timezone(&chrono_tz::Asia::Kolkata).format("%e %b"));
                // Log range

                // BUG: This function seems to stop incrementing after 6  
                let serial_num = sheets::get_next_empty_row(&ctx.data().secret_store, range.as_str(), spreadsheet_id.as_str()).await.unwrap();
                // Log serial_num

                // Lots of unwrap() here since they are hardcoded records and
                // shouldn't fail unless the OS does
                let row: sheets::Row = sheets::Row {
                    serial_number: serial_num,
                    name: record.get(1).unwrap(),
                    roll_number: record.get(2).unwrap(),
                    seat_number,
                    time_in: time_in_unwrapped.as_str(),
                    time_out: time_out_unwrapped.as_str(),
                };
                // Log row

                // CAUTION: Should handle this error safely
                let hub = sheets::build_hub(&ctx.data().secret_store).await.unwrap();
                // Log sucess

                match sheets::append_values_to_sheet(spreadsheet_id.as_str(), hub, ValueRange::from(row)).await {
                    // Send appended data and/or log errors/sucess
                    Ok(_) => {
                        let message = row.pretty_print();
                        ctx.reply(message).await?;
                        // Log sucess
                        return Ok(());
                        // Log sucess
                    }
                    Err(_) => ctx.reply(String::from("No")).await?,
                    // Log sucess in failing 
                };
            } else {
                ctx.reply(format!("Could not find {}'s data", author)).await?;
                // Log success in failing
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
    // Log sucess in building
    // framework.run().await.unwrap();
    Ok(framework.into())
}

