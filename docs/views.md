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

   Conditions include `p1` to `p4`, `#Label`, `$Goal`, `project: Name`, `area: Name`, `open`, `completed`, `due`, `deadline`, `created` and `touched` (with `before:`, `after:` and `within: D to D`), and bare words for a title search. Dates are relative (`today`, `+3d`, `-2w`) or `YYYY-MM-DD`.
3. Set grouping, sort, layout and the fields shown from the view's controls. The choice is saved with the view.
4. Pin a view to the sidebar, hide it, or use its menu to rename or delete it. Built-in views cannot be renamed or deleted; **Reset** restores their defaults.

## Verify it worked

Open the view: only tasks matching the filter are listed, and editing a task so it stops matching removes it from the list.

## Troubleshooting

**The text filter will not parse.** The error names the problem. Quote names that contain `&`, `|` or spaces: `project: "A & B"`. `@Name` is not supported in filters because it cannot tell an Area from a Project; use `area:` or `project:`.

**An empty view.** A view with no conditions shows everything in its scope. Check that the scope is the Area or Project you expect.

## Next

[Pages](pages.md): write plans and notes next to your tasks.
