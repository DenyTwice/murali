use tokio;
use dotenv::dotenv;
use poise::serenity_prelude as serenity;

// Bot storage
struct Data {} 

// Custom Error type that points to generic that implements error::Error AND 
// Send, Sync which are thread-safety traits
type Error = Box<dyn std::error::Error + Send + Sync>;

// Context holds most of the runtime information such as the user which invoked a command 
// and has methods implemented on it that performs actions such as sending a message
type Context<'a> = poise::Context<'a, Data, Error>;
#[poise::command(slash_command)]
async fn att(ctx: Context<'_>, message: Option<u32>) -> Result<(), Error> {
    ctx.say(String::from("WIP")).await?;
    Ok(())
}

#[tokio::main]
async fn main() {
    dotenv().ok();

    let framework_options = poise::FrameworkOptions {
            commands: vec![att()],
            prefix_options: poise::PrefixFrameworkOptions {
                prefix: Some(String::from(".")),
                ..Default::default()
            },
            ..Default::default()
    };
    let token = std::env::var("DISCORD_TOKEN").expect("missing DISCORD_TOKEN"); // !WARNING: Do NOT expose Discord Bot Token
    
    let framework = poise::Framework::builder()
        .options(framework_options)
        .token(token)
        .intents(serenity::GatewayIntents::non_privileged())
        .setup(|ctx, _ready, framework| {
            Box::pin(async move {
                poise::builtins::register_globally(ctx, &framework.options().commands).await?;
                Ok(Data {})
            })
        });

    framework.run().await.unwrap();
}

