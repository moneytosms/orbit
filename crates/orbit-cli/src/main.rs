mod cmd;
mod docs;
mod guide;
mod mcp;
mod out;
mod refs;
mod updates;

use clap::{CommandFactory, Parser, Subcommand};
use out::{Done, print_err, print_ok};
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser)]
#[command(
    name = "orbit",
    version,
    about = "Orbit from the terminal",
    after_help = "Examples:\n  orbit add \"buy milk\"\n  orbit ls\n  orbit done 1\n  orbit guide"
)]
struct Cli {
    /// Print one JSON object per result, with `"v":1`, instead of text.
    #[arg(long, global = true)]
    json: bool,
    /// Disable ANSI colors.
    #[arg(long, global = true)]
    no_color: bool,
    #[command(subcommand)]
    command: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// Sign this device in to a server. The password is read from ORBIT_PASSWORD or stdin.
    #[command(
        after_help = "Examples:\n  orbit login https://orbit.example.com\n  orbit login https://orbit.example.com --totp 123456\n  echo \"$PASSWORD\" | orbit login https://orbit.example.com --name laptop"
    )]
    Login {
        url: String,
        /// The current two-factor code, if the account has one.
        #[arg(long)]
        totp: Option<String>,
        #[arg(long, default_value = "orbit-cli")]
        name: String,
    },
    /// Sign this device out. Unsynced changes wait for the next login.
    #[command(after_help = "Examples:\n  orbit logout --dry-run\n  orbit logout --yes")]
    Logout {
        /// Log out even with unsynced changes.
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        dry_run: bool,
    },
    /// Send queued changes to the server and fetch the rest.
    #[command(after_help = "Examples:\n  orbit sync\n  orbit --json sync")]
    Sync,
    /// Show the server, sign-in state and unsynced count.
    #[command(after_help = "Examples:\n  orbit status\n  orbit --json status")]
    Status,
    /// Check for and install a verified CLI release.
    #[command(after_help = "Examples:\n  orbit self-update\n  orbit self-update --dry-run")]
    SelfUpdate {
        /// Download and verify the update without replacing this executable.
        #[arg(long)]
        dry_run: bool,
    },
    /// Add a task. Quick-add syntax applies: dates, p1-p4, #label, @Area or @project.
    #[command(
        after_help = "Examples:\n  orbit add \"buy milk\"\n  orbit add call the dentist tomorrow 9am p2 #health\n  orbit add --id 3f2b9c1e-6d4a-4b8e-9f1a-2c7d5e8b0a11 \"buy milk\" --dry-run"
    )]
    Add {
        #[arg(required = true)]
        title: Vec<String>,
        /// Use this UUID for the task. Adding an id that exists is a no-op.
        #[arg(long)]
        id: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// List open tasks in a terminal-width-aware table.
    #[command(
        after_help = "Examples:\n  orbit ls\n  orbit ls --all\n  orbit ls --wide\n  orbit --json ls"
    )]
    Ls {
        #[arg(long)]
        all: bool,
        /// Show ids and descriptions without truncating titles.
        #[arg(long)]
        wide: bool,
    },
    /// Show what to do next, with a reason for each item.
    #[command(
        after_help = "Examples:\n  orbit next\n  orbit next --all\n  orbit next --wide\n  orbit --json next"
    )]
    Next {
        #[arg(long)]
        all: bool,
        #[arg(long)]
        wide: bool,
    },
    /// Complete a task.
    #[command(
        after_help = "Examples:\n  orbit done 1\n  orbit done 3f2b9c\n  orbit done 1 --dry-run"
    )]
    Done {
        /// A row number from the device listing, id prefix, or unique title words.
        r: String,
        #[arg(long)]
        dry_run: bool,
    },
    /// Show one task.
    #[command(
        after_help = "Examples:\n  orbit show 1\n  orbit show 3f2b9c\n  orbit show buy milk\n  orbit --json show 1"
    )]
    Show { r: String },
    /// Change a task's title or description.
    #[command(
        after_help = "Examples:\n  orbit edit 1 --title \"buy oat milk\"\n  orbit edit 3f2b9c --description \"two litres\"\n  orbit edit 1 --title \"buy oat milk\" --dry-run"
    )]
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
    #[command(
        after_help = "Examples:\n  orbit search milk\n  orbit search dentist tomorrow\n  orbit search milk --all\n  orbit search milk --wide"
    )]
    Search {
        #[arg(required = true)]
        words: Vec<String>,
        #[arg(long)]
        all: bool,
        #[arg(long)]
        wide: bool,
    },
    /// Bring data in from another app.
    #[command(
        after_help = "Examples:\n  orbit import todoist\n  orbit import todoist --map 2201=project:Home/Garden --yes"
    )]
    Import {
        #[command(subcommand)]
        from: Import,
    },
    /// Check a routine in for a day (default today).
    #[command(
        after_help = "Examples:\n  orbit checkin 1\n  orbit checkin 1 --date 2026-11-19\n  orbit checkin 1 --dry-run"
    )]
    Checkin {
        r: String,
        /// YYYY-MM-DD.
        #[arg(long)]
        date: Option<String>,
        #[arg(long)]
        dry_run: bool,
    },
    /// Download everything as a zip: items.json, Pages as .md and attachments.
    #[command(
        after_help = "Examples:\n  orbit export orbit-export.zip\n  orbit --json export backup.zip"
    )]
    Export {
        /// Where to write the zip.
        file: PathBuf,
    },
    /// Print a short reference: commands, refs, listings, JSON shapes and exit codes.
    #[command(after_help = "Examples:\n  orbit guide\n  orbit --json guide")]
    Guide,
    /// Run as an MCP server over stdio, so AI tools can use Orbit.
    #[command(after_help = "Examples:\n  orbit mcp\n  claude mcp add orbit -- orbit mcp")]
    Mcp,
    /// Print shell completions to stdout.
    #[command(
        after_help = "Examples:\n  orbit completions bash > ~/.local/share/bash-completion/completions/orbit\n  orbit completions zsh > \"${fpath[1]}/_orbit\"\n  orbit completions fish > ~/.config/fish/completions/orbit.fish"
    )]
    Completions {
        /// The shell to write completions for.
        shell: clap_complete::Shell,
    },
    /// Write docs/cli.md and the man pages (maintainers; see `just docs`).
    #[command(
        hide = true,
        after_help = "Examples:\n  orbit docs docs\n  orbit docs /tmp/orbit-docs"
    )]
    Docs {
        /// The directory to write `cli.md` and `man/` into.
        dir: PathBuf,
    },
}

#[derive(Subcommand)]
enum Import {
    /// Import active Todoist items. The token is read from ORBIT_TODOIST_TOKEN or stdin, used once
    /// and never stored. Without --yes it only previews the mapping.
    #[command(
        after_help = "Examples:\n  orbit import todoist\n  orbit import todoist --map 2201=project:Home/Garden\n  orbit import todoist --map 2201=inbox --yes"
    )]
    Todoist {
        /// Change where a Todoist project goes: ID=area:NAME, ID=project:AREA/NAME or ID=inbox.
        #[arg(long = "map")]
        maps: Vec<String>,
        /// Import for real.
        #[arg(long)]
        yes: bool,
    },
}

fn run(command: Cmd, no_color: bool) -> Done {
    match command {
        Cmd::Login { url, totp, name } => cmd::login(&url, totp.as_deref(), &name),
        Cmd::Logout { yes, dry_run } => cmd::logout(yes, dry_run),
        Cmd::Sync => cmd::sync(),
        Cmd::Status => cmd::status(),
        Cmd::SelfUpdate { dry_run } => cmd::self_update(dry_run),
        Cmd::Add { title, id, dry_run } => cmd::add(&title, id.as_deref(), dry_run),
        Cmd::Ls { all, wide } => cmd::ls(all, wide, no_color),
        Cmd::Next { all, wide } => cmd::next(all, wide, no_color),
        Cmd::Done { r, dry_run } => cmd::done(&r, dry_run),
        Cmd::Show { r } => cmd::show(&r),
        Cmd::Edit {
            r,
            title,
            description,
            dry_run,
        } => cmd::edit(&r, title.as_deref(), description.as_deref(), dry_run),
        Cmd::Search { words, all, wide } => cmd::search(&words, all, wide, no_color),
        Cmd::Import {
            from: Import::Todoist { maps, yes },
        } => cmd::import_todoist(&maps, yes),
        Cmd::Checkin { r, date, dry_run } => cmd::checkin(&r, date.as_deref(), dry_run),
        Cmd::Export { file } => cmd::export(&file),
        Cmd::Guide => guide::guide(),
        Cmd::Docs { dir } => docs::write(&dir),
        Cmd::Mcp | Cmd::Completions { .. } => unreachable!("handled in main"),
    }
}

fn main() -> ExitCode {
    if std::env::args().len() == 1 {
        let mut cmd = Cli::command();
        let _ = cmd.print_help();
        println!();
        return ExitCode::SUCCESS;
    }
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(e) => {
            if !e.use_stderr() {
                let _ = e.print();
                return ExitCode::SUCCESS;
            }
            let json = std::env::args().any(|a| a == "--json");
            let text = e.to_string();
            let message = text.strip_prefix("error: ").unwrap_or(&text);
            let failure = out::usage(message);
            print_err(json, &failure);
            return ExitCode::from(failure.exit);
        }
    };
    let json = cli.json;
    let command = match cli.command {
        Cmd::Mcp => return mcp::serve(),
        Cmd::Completions { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "orbit", &mut std::io::stdout());
            return ExitCode::SUCCESS;
        }
        command => command,
    };
    match run(
        command,
        cli.no_color || std::env::var_os("NO_COLOR").is_some(),
    ) {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Every command and subcommand (hidden ones too) shows examples in its help.
    fn walk(cmd: &clap::Command, path: &str, missing: &mut Vec<String>) {
        for sub in cmd.get_subcommands().filter(|s| s.get_name() != "help") {
            let name = format!("{path} {}", sub.get_name());
            let help = sub.clone().render_long_help().to_string();
            if !help.contains("Examples") {
                missing.push(name.clone());
            }
            walk(sub, &name, missing);
        }
    }

    #[test]
    fn every_command_help_has_examples() {
        let mut root = Cli::command();
        root.build();
        let mut missing = vec![];
        if !root
            .clone()
            .render_long_help()
            .to_string()
            .contains("Examples")
        {
            missing.push("orbit".to_string());
        }
        walk(&root, "orbit", &mut missing);
        assert!(missing.is_empty(), "no examples in help for: {missing:?}");
    }
}
