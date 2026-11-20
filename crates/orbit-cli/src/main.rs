mod cmd;
mod out;
mod refs;

use clap::{Parser, Subcommand};
use out::{Done, print_err, print_ok};
use std::process::ExitCode;

#[derive(Parser)]
#[command(name = "orbit", version, about = "Orbit from the terminal")]
struct Cli {
    /// Print one JSON object per result, with `"v":1`, instead of text.
    #[arg(long, global = true)]
    json: bool,
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Sign this device in to a server. The password is read from ORBIT_PASSWORD or stdin.
    Login {
        url: String,
        /// The current two-factor code, if the account has one.
        #[arg(long)]
        totp: Option<String>,
        #[arg(long, default_value = "orbit-cli")]
        name: String,
    },
    /// Sign this device out. Unsynced changes wait for the next login.
    Logout {
        /// Log out even with unsynced changes.
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        dry_run: bool,
    },
    /// Send queued changes to the server and fetch the rest.
    Sync,
    /// Show the server, sign-in state and unsynced count.
    Status,
    /// Add a task.
    Add {
        #[arg(required = true)]
        title: Vec<String>,
        /// Use this UUID for the task. Adding an id that exists is a no-op.
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// List open tasks.
    Ls {
        #[arg(long)]
        all: bool,
    },
    /// Show what to do next.
    Next {
        #[arg(long)]
        all: bool,
    },
    /// Complete a task.
    Done {
        /// A row number from the last listing, or an id prefix of 6+ characters.
        r: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Show one task.
    Show { r: String },
    /// Change a task's title or description.
    Edit {
        r: String,
        #[arg(long)]
        title: Option<String>,
        #[arg(long)]
        description: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Find tasks by words.
    Search {
        #[arg(required = true)]
        words: Vec<String>,
        #[arg(long)]
        all: bool,
    },
    /// Bring data in from another app.
    Import {
        #[command(subcommand)]
        from: Import,
    },
    /// Check a routine in for a day (default today).
    Checkin {
        r: String,
        /// YYYY-MM-DD.
        #[arg(long)]
        date: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum Import {
    /// Import active Todoist items. The token is read from ORBIT_TODOIST_TOKEN or stdin, used once
    /// and never stored. Without --yes it only previews the mapping.
    Todoist {
        /// Change where a Todoist project goes: ID=area:NAME, ID=project:AREA/NAME or ID=inbox.
        #[arg(long = "map")]
        maps: Vec<String>,
        /// Import for real.
        #[arg(long)]
        yes: bool,
    },
}

fn run(command: Cmd) -> Done {
    match command {
        Cmd::Login { url, totp, name } => cmd::login(&url, totp.as_deref(), &name),
        Cmd::Logout { yes, dry_run } => cmd::logout(yes, dry_run),
        Cmd::Sync => cmd::sync(),
        Cmd::Status => cmd::status(),
        Cmd::Add { title, id, dry_run } => cmd::add(&title, id.as_deref(), dry_run),
        Cmd::Ls { all } => cmd::ls(all),
        Cmd::Next { all } => cmd::next(all),
        Cmd::Done { r, dry_run } => cmd::done(&r, dry_run),
        Cmd::Show { r } => cmd::show(&r),
        Cmd::Edit {
            r,
            title,
            description,
            dry_run,
        } => cmd::edit(&r, title.as_deref(), description.as_deref(), dry_run),
        Cmd::Search { words, all } => cmd::search(&words, all),
        Cmd::Import {
            from: Import::Todoist { maps, yes },
        } => cmd::import_todoist(&maps, yes),
        Cmd::Checkin { r, date, dry_run } => cmd::checkin(&r, date.as_deref(), dry_run),
    }
}

fn main() -> ExitCode {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            if !e.use_stderr() {
                let _ = e.print();
                return ExitCode::SUCCESS;
            }
            let json = std::env::args().any(|a| a == "--json");
            let text = e.to_string();
            let message = text
                .trim_start_matches("error: ")
                .lines()
                .next()
                .unwrap_or("bad arguments");
            let failure = out::usage(message);
            print_err(json, &failure);
            return ExitCode::from(failure.exit);
        }
    };
    let json = cli.json;
    match run(cli.command) {
        Ok((human, body)) => {
            print_ok(json, &human, body);
            ExitCode::SUCCESS
        }
        Err(failure) => {
            print_err(json, &failure);
            ExitCode::from(failure.exit)
        }
    }
}
