# Quick add and tasks

For daily users who want to capture a task in one line and know what Orbit understands. The same parser runs in the web app, the desktop capture overlay, the Android widget and `orbit add`.

## Before you start

- Orbit installed on at least one device ([Install](install.md)) and signed in.
- Know the names of your Areas, Projects, Goals, Pages and Labels. Names are matched case-insensitively and keep their stored casing.

## Steps

1. Type a title and add tokens anywhere in the line. The parser removes recognised tokens from the title:

   | Token | Meaning | Example |
   | --- | --- | --- |
   | `today`, `tomorrow`, `fri`, `in 3 days`, `next week`, `dec 5`, `2026-12-01` | due date (`next week` is next Monday; a weekday is its next occurrence) | `pay rent fri` |
   | `6pm`, `18:00`, `9:15am` | due time (today if no date) | `call mom tomorrow 6pm` |
   | `p1` to `p4` | priority, last one wins | `fix bug p1` |
   | `#label` | label | `buy milk #errand` |
   | `@Name` | Area or Project | `read chapter @College` |
   | `$Name` | Goal | `run 5k $Get fit` |
   | `%Name` | Page | `book flights %Trip notes` |
   | `every day`, `every mon,wed`, `every! 3 weeks` | recurrence (`every!` counts from completion) | `water plants every 3 days` |
   | `!30m`, `!9am`, `!fri 9am` | reminder (offset before due, or a fixed time) | `dentist fri 3pm !1h` |

2. Use a backslash to keep a sigil literal: `issue \#1` stays `issue #1`.
3. An unknown name stays in the title and shows as unresolved instead of guessing.
4. Organise later. Captures with no destination land in the Inbox. Move a task by dragging it, from its menu, or by editing it in the task drawer.
5. Group tasks inside an Area or a Project with **Sections**: use **+ New section** at the end of the list. Drag tasks between sections and drag headings to reorder. Deleting a section moves its tasks to the parent list, never deletes them.

## Verify it worked

```sh
orbit add "call the dentist tomorrow 9am p2 #health"
orbit ls
```

The task is listed with tomorrow's date, priority 2 and the `health` label, and its title is `call the dentist`.

## Troubleshooting

**A word became part of the title instead of a date.** Only whole phrases count: `may` and bare numbers are not dates. Write `may 5`, an ISO date, or a weekday.

**`@Name` did not resolve.** The name must match an existing Area or Project and end at a word boundary. Multi-word names match the longest name first.

**A task I added offline is missing on another device.** Changes wait in the queue until the next sync. See [Install, Troubleshooting](install.md#troubleshooting).

## Next

[Views and filters](views.md): see the tasks you captured the way you want.
