# Orbit

A self-hostable personal planner: capture tasks fast, and connect them to the goals and plans they serve.

## Language

**Task**:
A single actionable thing to do, optionally with a due date, priority, labels and a recurrence.
_Avoid_: To-do, item (when the kind is known), card

**Project**:
A container where tasks live, with no end state of its own.
_Avoid_: List, board, area

**Goal**:
An outcome with a done-condition and an optional target date that tasks and milestones count toward.
_Avoid_: Objective, OKR, roadmap (a roadmap is a Goal's view, not a separate thing)

**Routine**:
A habit checked in on a schedule, either ongoing or time-boxed to a number of days; a missed day is missed, never overdue.
_Avoid_: Habit (in UI copy is fine, not in code), recurring task, streak

**Check-in**:
One day's record of a Routine: done, skipped, or missed.
_Avoid_: Completion, log entry

**Recurring task**:
A Task that comes back on its recurrence after completion and stays overdue until done.
_Avoid_: Routine, repeat

**Page**:
A standalone markdown document, optionally with attachments, that can be linked to goals, projects and tasks.
_Avoid_: Note, doc

**Stardust**:
The points a user earns by completing tasks and goals; subtle, never penalising.
_Avoid_: Karma, XP, score

**Quick capture**:
Adding a task in one short step from any surface (widget, share sheet, hotkey, quick-add, CLI), organised later.
_Avoid_: Quick add (that is the in-app input box only)

**Inbox**:
The default Project that captured tasks land in until organised.
