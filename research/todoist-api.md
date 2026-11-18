# Todoist API for active-item import

Issue: moneytosms/orbit#6. Date: 2026-11-19.
Scope: one-time import of active projects, sections, labels, tasks, comments.
Method: official developer.todoist.com `.md` pages (index at https://developer.todoist.com/llms.txt), OpenAPI spec, Doist developer group post. Facts were read via a summarizing fetch, so quote-check before writing code. No live API calls made.

## Summary

- Use API v1 only. Base `https://api.todoist.com/api/v1/`. Sync v9 and REST v2 were shut down 2026-02-10 ([Doist announcement, 2025-12-04](https://groups.google.com/a/doist.com/g/todoist-api/c/brwENjfT_tk)).
- Auth: personal token, `Authorization: Bearer <token>`, from https://app.todoist.com/app/settings/integrations ([authorization.md](https://developer.todoist.com/api/v1/authorization.md)).
- Active-only listings are native: `/tasks`, `/projects`, `/sections`, `/labels`, `/reminders`. Archived data has separate endpoints.
- Comments have no bulk list. Must call per task or per project (`task_id` xor `project_id`).
- Nothing active is lost in REST, except saved filters (Sync-only). Karma is exposed.

## Endpoints

All paginated list endpoints: `cursor` + `limit` (default 50, max 200). Done when `next_cursor` is null ([pagination.md](https://developer.todoist.com/api/v1/pagination.md)).

| Data | Endpoint | Notes | Source |
|---|---|---|---|
| Tasks | `GET /tasks` | Active only. Params: `project_id`, `section_id`, `parent_id`, `label` (name), `ids`. | [tasks.md](https://developer.todoist.com/api/v1/tasks.md) |
| Projects | `GET /projects` | Active only. Archived: `GET /projects/archived`. | [projects.md](https://developer.todoist.com/api/v1/projects.md) |
| Sections | `GET /sections` | Active only (`project_id` or `workspace_id`). Archived: `GET /sections/archived`. | [sections.md](https://developer.todoist.com/api/v1/sections.md), [migrating-from-v9.md](https://developer.todoist.com/api/v1/migrating-from-v9.md) |
| Labels | `GET /labels` | Personal labels. Shared: `GET /labels/shared` (names only). | [labels.md](https://developer.todoist.com/api/v1/labels.md) |
| Comments | `GET /comments?task_id=` or `?project_id=` | Exactly one param required. | [comments.md](https://developer.todoist.com/api/v1/comments.md) |
| Reminders | `GET /reminders` | Active. Filter `task_id`. Plan-dependent. | [reminders.md](https://developer.todoist.com/api/v1/reminders.md) |
| Me | `GET /user` | `tz_info.timezone` for user zone. | [user.md](https://developer.todoist.com/api/v1/user.md) |

Completed tasks not needed for active import. For reference: `/tasks/completed/by_completion_date` (max 3-month range), `/tasks/completed/by_due_date` (max 6-week range).

## Field lists

**Task**: `id`, `user_id`, `project_id`, `section_id` (nullable), `parent_id` (nullable), `content`, `description`, `labels` (array of label **names**, not IDs), `priority` (int), `due` (nullable), `deadline` (nullable), `duration` (nullable), `child_order`, `day_order`, `order_key`, `checked` (bool), `is_collapsed`, `is_deleted`, `added_at`, `updated_at`, `completed_at`, `added_by_uid`, `assigned_by_uid`, `responsible_uid`, `completed_by_uid`, `completed_count`, `postponed_count`, `note_count` (deprecated, always 0).
- `due`: `date`, `string`, `lang`, `is_recurring`. Recurrence is `due.string`; writes use `due_string`. No per-task timezone in `due` examples.
- `deadline`: `date`, `lang`. `duration`: `amount`, `unit`.
- Absent: `creator`, `assignee` (use `added_by_uid`, `responsible_uid`), `comment_count`, `url`.

**Project**: `id`, `name`, `description`, `parent_id`, `child_order`, `is_collapsed`, `is_shared`, `is_favorite`, `inbox_project` (not `is_inbox_project`), `view_style`, `color`, `icon`, `is_archived`, `is_deleted`, `is_frozen`, `created_at`, `updated_at`, `creator_uid`, `order_key`, `default_order`, `public_key`, `access`, `role`. Workspace-project schema was truncated in the fetch; `workspace_id`/`folder_id` seen only on workspace variants.

**Section**: `id`, `user_id`, `project_id`, `name`, `description`, `section_order`, `order_key`, `is_collapsed`, `is_archived`, `is_deleted`, `added_at`, `updated_at`, `archived_at`. Flat: no parent-section field. Position is `section_order`.

**Label**: `id`, `name`, `color` (name, e.g. `berry_red`), `order`, `is_favorite`. No `is_deleted` in schema.

**Comment**: `id`, `posted_uid`, `content`, `file_attachment` (object or null), `uids_to_notify`, `is_deleted`, `posted_at`, `reactions`. No `task_id`/`project_id` on object; the parent comes from the request.

**Reminder**: `id`, `item_id` (task), `notify_uid`, `is_deleted`, `is_urgent`, `type` (`absolute`|`relative`), `minute_offset`, `due`.

## Priority

API value 4 = app "p1" (very urgent). API 1 = app "p4" (natural). Source: [sync/tasks.md](https://developer.todoist.com/api/v1/sync/tasks.md): "`4` for very urgent and `1` for natural ... `p1` will return `4` in the API."

**Conflict**: [tasks.md](https://developer.todoist.com/api/v1/tasks.md) create/update schema says "1 is highest", response schema says "1 (normal) to 4 (urgent)". Follow sync/tasks.md. Confirm with one live call.

## Auth and limits

- Personal token: no expiry or revocation documented (absence, not a guarantee). OAuth access tokens expire after 1h ([overview](https://developer.todoist.com/api/v1/)). Personal tokens can migrate to OAuth via `POST /access_tokens/migrate_personal_token`.
- Documented rate limits ([request-limits.md](https://developer.todoist.com/api/v1/request-limits.md)) are Sync-only: 1000 partial-sync and 100 full-sync requests per user per 15 min. Max 100 commands per batch request. 1 MiB POST body limit.
- REST per-endpoint limits: **not documented** on the pages read. 429 exists (OpenAPI lists 429 "Rate limit exceeded", seen on a template endpoint in the fetched part of [openapi.json](https://developer.todoist.com/openapi.json)). Overview mentions `retry_after` in `error_extra` and a `Retry-After` header. **UNVERIFIED** for list endpoints. Plan: backoff on 429, honor `Retry-After`, measure live.

## Lost or gated

- **Filters** (saved searches): no REST endpoint. Only via Sync `POST /api/v1/sync` with `resource_types=["filters"]` ([sync/filters.md](https://developer.todoist.com/api/v1/sync/filters.md)). Sync is POST-only. Fields: `id`, `name`, `query`, `description`, `color`, `item_order`, `order_key`, `is_deleted`, `is_favorite`, `is_frozen`.
- **Reminders**: REST, plan-dependent. Plan names not stated.
- **Karma**: `karma`, `karma_trend`, `completed_count` on `GET /user` and `GET /tasks/completed/stats` ([user.md](https://developer.todoist.com/api/v1/user.md)). Account-level, not per item.
- **Renames**: items to tasks, notes to comments, `collapsed` to `is_collapsed`. `day_orders` removed from sync response.
- **IDs**: opaque strings. Old v2 IDs rejected. `/ids_mapping` exists for legacy IDs only.

## Unverified (test live before relying on it)

1. Does `GET /tasks` include subtasks? Implied via `parent_id`, not stated.
2. Does `GET /tasks` exclude tasks inside archived projects or sections? Not stated.
3. Does `GET /comments?project_id=` return task comments too, or only project-level ones? Not stated. If not, comments need one call per task. `note_count` is deprecated (always 0), so no way to pre-filter tasks with comments.
4. Shape and download URL of `file_attachment`. Not documented.
5. REST rate limits and 429 body/headers.
6. Due-date timezone semantics (floating vs fixed) for `due.date` datetimes.
7. Priority direction (see Priority conflict).
8. Personal-token lifetime.

Not found: an official API v1 changelog page (`/api/v1/changelog` returned 404).

## Sources

- https://developer.todoist.com/api/v1/ (base URL, auth, deprecation notices)
- https://developer.todoist.com/llms.txt (page index)
- https://developer.todoist.com/api/v1/tasks.md
- https://developer.todoist.com/api/v1/projects.md
- https://developer.todoist.com/api/v1/sections.md
- https://developer.todoist.com/api/v1/labels.md
- https://developer.todoist.com/api/v1/comments.md
- https://developer.todoist.com/api/v1/reminders.md
- https://developer.todoist.com/api/v1/filters.md
- https://developer.todoist.com/api/v1/user.md
- https://developer.todoist.com/api/v1/request-limits.md
- https://developer.todoist.com/api/v1/pagination.md
- https://developer.todoist.com/api/v1/authorization.md
- https://developer.todoist.com/api/v1/migrating-from-v9.md
- https://developer.todoist.com/api/v1/sync/overview.md
- https://developer.todoist.com/api/v1/sync/tasks.md
- https://developer.todoist.com/api/v1/sync/filters.md
- https://developer.todoist.com/openapi.json
- https://groups.google.com/a/doist.com/g/todoist-api/c/brwENjfT_tk (shutdown, 2026-02-10)

## Next

Spike: one personal token, `GET /api/v1/tasks?limit=5`, `GET /comments?project_id=...`, and a 429 probe. Resolves unverified 1-3, 5, 7.
