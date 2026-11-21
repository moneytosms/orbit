<script lang="ts">
  import { Button } from '$lib/components/ui/button'
  import { Input } from '$lib/components/ui/input'
  import { commitImport, startImport, type Plan, type Preview, type Report } from '$lib/todoist'

  let token = $state('')
  let busy = $state(false)
  let error = $state('')
  let job = ''
  let preview: Preview | undefined = $state()
  let report: Report | undefined = $state()

  async function load(e: SubmitEvent) {
    e.preventDefault()
    busy = true
    error = ''
    report = undefined
    try {
      ;({ job, preview } = await startImport(token))
    } catch (err) {
      error = err instanceof Error ? err.message : "Couldn't read Todoist."
    } finally {
      // The token was used once by the server; don't keep it here either.
      token = ''
      busy = false
    }
  }

  async function commit() {
    if (!preview) return
    busy = true
    error = ''
    try {
      report = await commitImport(job, preview.projects)
      preview = undefined
    } catch {
      error = "Couldn't import. Paste the token again to retry."
    } finally {
      busy = false
    }
  }

  const select =
    'h-[2.125rem] rounded-md border border-line bg-surface px-2 text-sm text-text shadow-1 outline-none focus-visible:border-accent-line'
  const fresh = (p: Plan) => p.tasks - p.imported
</script>

<section class="panel">
  <h2 class="panel-title">Import from Todoist</h2>
  {#if !preview}
    <form onsubmit={load} class="flex max-w-sm flex-col gap-3">
      <p class="text-muted">
        Brings over active projects, sections, tasks, comments, files and reminders. The token is used once and never
        stored.
      </p>
      <Input
        type="password"
        bind:value={token}
        placeholder="Todoist API token"
        aria-label="Todoist API token"
        autocomplete="off"
        required
      />
      <Button type="submit" class="self-start" disabled={busy}>{busy ? 'Reading Todoist…' : 'Preview import'}</Button>
    </form>
  {:else}
    <p class="mb-3 text-muted">
      {preview.totals.tasks} tasks, {preview.totals.sections} sections, {preview.totals.labels} labels. Change where each
      project goes, then import. Items already imported are skipped.
    </p>
    <ul>
      {#each preview.projects as p (p.id)}
        <li class="flex flex-wrap items-center gap-2 border-b border-line py-2 last:border-0">
          <div class="min-w-40 flex-1">
            <div class="truncate font-medium">{p.path}</div>
            <div class="text-[0.7812rem] text-muted">{fresh(p)} new of {p.tasks} tasks</div>
          </div>
          <select bind:value={p.kind} aria-label="Becomes, {p.path}" class={select}>
            <option value="area">Area</option>
            <option value="project">Project</option>
            <option value="inbox">Inbox</option>
          </select>
          {#if p.kind !== 'inbox'}
            <Input bind:value={p.area} class="w-36" aria-label="Area, {p.path}" />
          {/if}
          {#if p.kind === 'project'}
            <Input bind:value={p.name} class="w-36" aria-label="Project, {p.path}" />
          {/if}
        </li>
      {/each}
    </ul>
    <div class="mt-3 flex gap-2">
      <Button variant="primary" disabled={busy} onclick={commit}>{busy ? 'Importing…' : 'Import'}</Button>
      <Button disabled={busy} onclick={() => (preview = undefined)}>Cancel</Button>
    </div>
  {/if}
  {#if error}<p role="alert" class="mt-2 text-danger-text">{error}</p>{/if}
  {#if report}
    <div role="status" class="mt-3">
      <p>
        Imported {report.tasks} tasks, {report.sections} sections, {report.projects} projects, {report.areas} areas, {report.pages}
        pages and {report.reminders} reminders. {report.existing_tasks} tasks were already imported.
      </p>
      {#if report.issues.length}
        <p class="mt-2 font-medium">Came over changed, or not at all</p>
        <ul class="list-disc pl-5 text-muted">
          {#each report.issues as issue (issue)}<li>{issue}</li>{/each}
        </ul>
      {/if}
      <p class="mt-2 text-muted">Never imported: {report.dropped.join(', ')}.</p>
    </div>
  {/if}
</section>
