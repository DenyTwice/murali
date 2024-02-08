# Murali

Murali is a discord bot built by and for amFOSS-2023 members. Primarily, it automates the _extremely_ tedious procdeure of opening
google sheets to add an entry that marks your attendance. This is instead achieved through a simple discord command that takes in
your seat number, time you entered and the time you intend to leave.

# Prerequisities

Before running the bot, ensure you have the following set up:

- Rust: Make sure you have Rust installed on your system. You can install Rust by following the instructions on rustup.rs.
    - For deployment and local runs, you will also require an account on [shuttle.rs](https://www.shuttle.rs/).

- Google API Credentials: Required to perform actions on behalf of a service account. Create a project in the Google Cloud Console, 
enable the Google Sheets API, create a service account and download the JSON credentials file. Place this in a directory called 
secrets at the root of the project. 

- Discord Bot Token: Required to run the bot account on Discord. Create a Discord bot on the Discord Developer Portal, and add the 
token to your Secrets.toml as "DISCORD_TOKEN".

- Spreadsheet ID: The ID of the spreadsheet used for attendance. You can get the ID from the spreadsheet's URL. Add the ID 
to Secrets.toml as "SPREADSHEET_ID". 

- Template ID: The ID for the template sheet. Again, available from the URL. This is the sheet that will be duplicated if the bot does
not find a sheet made for the day (sheets are named after the day they correspond to). Add the ID to Secrets.toml as "TEMPLATE_ID". 

The final Secrets.toml file should resemble:
```
DISCORD_TOKEN="placeholdertoken"
SPREADSHEET_ID="placeholdertoken"
TEMPLATE_ID="placeholdertoken"
```

# Running

- Run `cargo shuttle login` first to login with your API KEY, found from [Shuttle.rs Console's](https://www.console.shuttle.rs) Profile 
overview page.

- Use `cargo shuttle run` to locally run the bot. This is useful to check whether everything has been setup properly before starting
development or before committing changes.

- Use `cargo shuttle project start --idle-minutes 0` to initialize a project on shuttle.

- Then run `cargo shuttle project deploy` to deploy the project.

# Discord Commands

As of the 8th of February, the only command is `att`.

Syntax: `/att [seat_number] [time_in] [time_out]`
Where square brackets imply optional arguments.

## Defaults:
- seat_number = ""
- time_in = "17:30"
- time_out = isMale ? "22:00" : "21:00" 

# License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
