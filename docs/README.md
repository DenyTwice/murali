# Murali

Murali is a discord bot built by and for amFOSS-2023 members. Primarily, it automates the _extremely_ tedious procdeure of opening
google sheets to add an entry that marks your attendance. This is instead achieved through a simple discord command that takes in
your seat number (and optionally the time you entered and the time you intend to leave).

# Prerequisities

Before running the bot, ensure you have the following set up:

- Rust: Make sure you have Rust installed on your system. You can install Rust by following the instructions on rustup.rs.
    - For deployment and local runs, you will also require an account on [shuttle.rs](https://www.shuttle.rs/).

- Google API Credentials: Create a project in the Google Cloud Console, enable the Google Sheets API, create a service account
and download the JSON credentials file. Place this in a directory called secrets and add the relative path to a file called Secrets.toml as
"SA_CREDENTIALS_PATH".

- Discord Bot Token: Create a Discord bot on the Discord Developer Portal, and add the token to your Secrets.toml as "DISCORD_TOKEN".

- Spreadsheet ID: Add the ID of the spreadsheet to Secrets.toml as "SPREADSHEET_ID" and modify the range variable to suit your needs
in the 'att(..)' function in main.rs

The final Secrets.toml file should resemble:
```
DISCORD_TOKEN=""
SA_CREDENTIALS_PATH="secrets/sa_credentials.json"
SPREADSHEET_ID=""
```

# Usage

``` /att [SEAT_NUMBER] <time_in> <time_out> ```

# License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.
