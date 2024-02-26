# Backup Maker 5000

This rust CLI program with an almost meme name is a simple backup tool that I made primarily for myself.
However, it can be useful for others interested in how to upload data to a GCS Bucket programmatically.
I personally use this to create offsite backups for Minecraft directories on a linux remote server, but this works for every directory.

**Warning**: This is not going to work out of the box for you. You need to have a GCP account and a GCS bucket to use this. 
You will also need to change some of the code for your personal needs. I highly recommend basic async Rust knowledge to understand the code.

## Features

- Uploads a directory to a GCS Bucket
- Can be used as a manually triggered backup tool or as a cron job
- Efficient compression and upload process
- Capability to exclude specific directories from the backup
- Nice progress bar
- Configurable verbosity for debugging purpose

## Pre-requisites

- A GCP account
- A GCS bucket
- A service account with the necessary permissions to upload to the bucket.
- A JSON key file for the service account
- Rust installed on your machine

### Obtaining a service account

This is not a guide on how to create a service account, but here are the basic steps:

1. Go to the [GCP Console](https://console.cloud.google.com/)
2. Go to the IAM & Admin section
3. Go to Service Accounts
4. Create a new service account
5. Give it the necessary permissions to upload to the bucket
6. Create a JSON key file for the service account
7. Save the JSON key file in a safe place
8. Use the JSON key file in the program
9. Profit

You should most definitly read up on [Google's IAM Documentation Guide](https://cloud.google.com/iam/docs/service-account-overview) on service accounts.

## Installation

Ensure that Rust is installed on your machine. If not, you can install it from [rustup.rs](https://rustup.rs/).
You can check if Rust is installed by running `rustc --version` in your terminal.

Clone the repository and navigate to the directory in your terminal.

```bash
git clone https://github.com/defnot001/backup_maker_5000.git
```

Rename the `config.example.json` file to `config.json` and fill in the necessary information.
Navigate to the directory and run the following command to build the program.

```bash
cargo build --release
```

The binary will be located in the `target/release` directory.

## Usage

Execute the program with the required server type as the first argument:

```bash
./target/release/backup_maker_5000 smp
```

### CLI Arguments:

The first argument is positional. It is the name of the server type. This is used to determine which directory to backup.

- `--exclude` | `-e`: Exclude a directory from the backup. This can be used multiple times.
- `--verbose` | `-v`: Enable verbose output. This can be used multiple times.
- `--help`: Show the help message.
- `--version`: Show the version of the program.
- `--about`: Show the about message.

The `--verbose` flag can be used multiple times to increase the verbosity of the output.

- No flag: Shows the progress bar.
- `-v`: Shows the progress bar and warnings.
- `-vv`: Shows the progress bar, warnings, and info level information.
- `-vvv`: Shows the progress bar, warnings, info level information, and debug level information. _(This spams the terminal pretty hard!)_

If you have troubles with the program, please contact me or open an issue on the GitHub repository.


