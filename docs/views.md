# Views and filters

For daily users who want a task list that shows only what matters now. A View is a saved configuration of scope, filters, grouping, sort, layout and fields.

## Before you start

- Some tasks in Orbit ([Quick add and tasks](tasks.md)).
- Built-in views need no setup: Home (the Dashboard), Inbox and Upcoming sit in the sidebar. Area and Project screens offer List, Board and Table layouts.

## Steps

1. Open the sidebar and choose **New view**. Open **Filter** on the new view for the visual editor: pick a field, an operator and a value, and group conditions with all/any.
2. Prefer typing? The same filter has a text form, with `&` for and, `|` for or, `!` for not and parentheses for groups:

   ```text
   p1 & due before: tomorrow & #errand
   ```

   Conditions include `p1` to `p4`, `#Label`, `$Goal`, `project: Name`, `area: Name`, `open`, `completed`, `due`, `deadline`, `created` and `touched` (with `before:`, `after:` and `within: D to D`), and bare words for a title search. Dates are relative (`today`, `+3d`, `-2w`, `-7 days`, `7 days ago`) or `YYYY-MM-DD`.

   More terms: `overdue` (or `od`), `recurring`, `subtask` (put `!` before `recurring` or `subtask` to flip them), `no date`, `no time`, `no labels`, `no priority` (same as `p4`), `date before: D` and `date after: D` (same as `due`), `created before: -7 days`, and `view all` (matches everything). `search: words` finds tasks whose title or description holds every word, in any order, and `*` is a wildcard inside a word (`search: fl*ts book`); `!search: words` excludes them. `title: text` matches the title only. `#work*` or `#*work` matches every label whose name fits the pattern. Orbit keeps its own sigils: `#` is a label, `@` an area or project, `$` a goal and `%` a page.
3. Set grouping, sort, layout and the fields shown from the view's controls. The choice is saved with the view.
   On an Inbox, Area or Project list, the **Group** and **Sort** menus above the list group tasks by date, priority or label and sort them by name, date, priority or date added. Each screen remembers its choice on this device. A grouped or re-sorted list is not draggable; set both back to **No grouping** and **Manual** to reorder by hand.
4. Pin a view to the sidebar, hide it, or use its menu to rename or delete it. Built-in views cannot be renamed or deleted; **Reset** restores their defaults.

5. In a Calendar layout, tick **Show future occurrences of recurring tasks** to see the next occurrences of repeating tasks as faded, read-only entries. Open one to edit the task itself.
6. Keyboard: with a task selected (`j`/`k`), `t` sets today, `y` tomorrow, **Shift T** clears the date, `1` to `4` set the priority, `e` completes and `c` or Enter opens it. Press `?` for the full list.

## Verify it worked

Open the view: only tasks matching the filter are listed, and editing a task so it stops matching removes it from the list.

## Troubleshooting

**The text filter will not parse.** The error names the problem. Quote names that contain `&`, `|` or spaces: `project: "A & B"`. `@Name` is not supported in filters because it cannot tell an Area from a Project; use `area:` or `project:`.

**An empty view.** A view with no conditions shows everything in its scope. Check that the scope is the Area or Project you expect.

## Next

[Pages](pages.md): write plans and notes next to your tasks.
