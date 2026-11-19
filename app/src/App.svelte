<script lang="ts">
  import { completeTask, createTask, inboxTasks, start, type Task } from './lib/sync'

  let tasks: Task[] = $state([])
  let title = $state('')
  const refresh = async () => (tasks = await inboxTasks())
  start(refresh)
  refresh()

  async function add(e: SubmitEvent) {
    e.preventDefault()
    if (!title.trim()) return
    await createTask(title)
    title = ''
  }
</script>

<main>
  <h1>Inbox</h1>
  <form onsubmit={add}>
    <input bind:value={title} placeholder="Add a task" aria-label="Task title" />
  </form>
  <ul>
    {#each tasks as task (task.id)}
      <li>
        <label><input type="checkbox" onchange={() => completeTask(task.id)} /> {task.title}</label>
      </li>
    {/each}
  </ul>
</main>
