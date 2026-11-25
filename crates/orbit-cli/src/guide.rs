//! `orbit guide`: a short reference for people and agents. Plain text, kept stable.
use crate::out::Done;
use serde_json::json;

const GUIDE: &str = "\
Orbit CLI guide

Data lives in ORBIT_DIR, else $XDG_DATA_HOME/orbit, else ~/.local/share/orbit.
Everything works offline. `orbit sync` sends queued changes and fetches the rest.

Core commands
  orbit add TITLE...             add a task (--id UUID to pick its id, --dry-run)
  orbit add -                    one task per stdin line (--lines TEXT does the same)
  orbit postpone REF [PHRASE]    move the due date, keeping its time (default tomorrow)
  orbit reschedule REF PHRASE    set the due date and time, or `none` to clear it
  orbit snooze REF [PHRASE]      remind later (default one hour from now)
  orbit overdue [--reschedule P] list tasks past their due date, or move them all
  orbit ls                       open tasks (--all for every row)
  orbit next                     what to do next (--all for every row)
  orbit show REF                 one task, with its id and status
  orbit done REF                 complete a task (--dry-run)
  orbit edit REF --title T --description D
  orbit search WORDS...          find tasks by words (--all)
  orbit checkin REF              check a routine in (--date YYYY-MM-DD, default today)
  orbit export --md [FILE]       tasks as a Markdown checklist (no server needed)
  orbit login URL, logout, sync, status
  orbit import todoist           preview an import (--map ID=TARGET, --yes to import)
  orbit mcp                      serve the same commands to AI tools over stdio
  orbit completions SHELL        shell completions for bash, zsh or fish

Refs
  A ref names a task or routine, either:
  - a row number from the device-wide last listing (ls, next, search), kept for 24 hours.
  - an id prefix (shown by --wide), or words matching a unique title.
  Unknown or ambiguous refs explain how to fix them; nothing is guessed.

Filters and listings
  There are no field filters yet (no due, priority or label flags).
  ls, next and search show 20 rows and say so when they truncate; --all shows every row.
  Tables fit the terminal; --wide shows ids and descriptions without truncation. NO_COLOR
  or --no-color disables ANSI colors.
  search needs words: each word must start a word of the task text, so `orbit search mil`
  finds \"buy milk\".

Dry runs
  add, done, edit, checkin, postpone, reschedule, snooze, overdue and logout take --dry-run:
  they report what would change and write nothing.

JSON (--json, a global flag)
  Each command prints one JSON object on stdout, with \"v\":1.
  Success: the command's own fields, e.g. add gives {\"v\":1,\"id\",\"title\",\"created\",\"dry_run\"}
  and show gives {\"v\":1,\"task\":{...}}.
  Listings (ls, next, search): {\"v\":1,\"items\":[{\"n\",\"id\",\"title\",\"detail\"}],\"total\",
  \"shown\",\"truncated\"}, plus \"hint\" when truncated.
  Failure: {\"v\":1,\"error\":{\"code\",\"message\",\"hint\"}} on stdout. The code is stable
  (for example not_found, ambiguous, signed_out, confirm).

Exit codes
  0 success. 1 a failed command (see the error code). 2 bad arguments.
";

pub fn guide() -> Done {
    Ok((GUIDE.trim_end().into(), json!({ "guide": GUIDE })))
}
