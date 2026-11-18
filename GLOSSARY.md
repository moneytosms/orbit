# Orbit

A self-hostable personal planner: capture tasks fast, and connect them to the goals and plans they serve.

## Language

**Task**:
A single actionable thing to do, optionally with a due date, priority, labels and a recurrence.
_Avoid_: To-do, item (when the kind is known), card

**Area**:
The top level of the tree: a part of life (Home, College, Career) that holds Projects, Sections, Tasks, Goals and Routines. Areas never nest.
_Avoid_: Domain, space, workspace, category

**Project**:
An optional container of Tasks inside an Area, with no end state of its own.
_Avoid_: List, board

**Goal**:
An outcome with a done-condition and an optional target date that tasks and milestones count toward. Its progress is a Count (done linked items out of all linked), a Target (a number reached by progress entries) or Manual (done or not).
_Avoid_: Objective, OKR, roadmap (a roadmap is a Goal's view, not a separate thing)

**Sub-goal**:
A Goal directly under another Goal, counting as one unit toward it; only one level deep.

**Milestone**:
A dated checkpoint inside exactly one Goal; it holds no Tasks.

**Section**:
A named group of Tasks inside a Project or an Area.

**Label**:
A global tag on Tasks, shared across all Areas.

**Routine**:
A habit checked in on a schedule, either ongoing or time-boxed to a number of days; a missed day is missed, never overdue.
_Avoid_: Habit (in UI copy is fine, not in code), recurring task, streak

**Check-in**:
One day's record of a Routine: done or skipped. A scheduled day that passes with no Check-in is missed.
_Avoid_: Completion, log entry

**Recurring task**:
A Task that comes back on its recurrence after completion and stays overdue until done.
_Avoid_: Routine, repeat

**Page**:
A markdown document, optionally with attachments, owned by one Area, Project or Goal; an owner can have many Pages.
_Avoid_: Note, doc

**Link**:
An inline reference from a Page or Task description to a Page, Task, Goal, Project or Area.

**Backlink**:
A Link seen from the thing it points at.

**Stardust**:
The points a user earns by completing tasks and goals; subtle, never penalising.
_Avoid_: Karma, XP, score

**Quick capture**:
Adding a task in one short step from any surface (widget, share sheet, hotkey, quick-add, CLI), organised later.
_Avoid_: Quick add (that is the in-app input box only)

**Inbox**:
The built-in Area where captures land when no destination is chosen; it holds only Tasks, cannot be deleted, and its Tasks never need filing.

**Archive**:
Hide an Area, Project, Goal or Routine (and everything inside it) from views, read-only and restorable.
_Avoid_: Complete (that is for Tasks, Milestones and Goals)

**Trash**:
Where deleted things wait 30 days before they are gone for good.

**Dashboard**:
The default start view: what's next, Routines due, Goal progress, streaks and stats.
_Avoid_: Today, home

**What's next**:
The Dashboard's ranked list of Tasks and Milestones that need attention soon, each with the reason it is there.
_Avoid_: Top 3, focus list

**Behind pace**:
A Goal whose progress is below a straight line from its start to its target date.

**View**:
A saved configuration of what to show (scope, filters, grouping, sort, layout, fields), such as the Dashboard, Calendar or a custom view.
_Avoid_: Filter, perspective, page
