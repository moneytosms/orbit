type Row = { id: string; title: string }

export const PROMPT_EXAMPLE = [
  '%%orbit:v1',
  'flowchart LR',
  '  %%orbit roadmap "Upskill"',
  '  %%orbit band p0 {"name":"Week 1","start":"2026-11-21","end":"2026-11-27"}',
  '  %%orbit node t1 {"band_id":"p0","x":120,"y":40}',
  '  %%orbit node d1 {"x":420,"y":40}',
  '  %%orbit node t2 {"x":720,"y":40}',
  '  %%orbit edge d1 -> t2 {"kind":"then","label":"if funded"}',
  '',
  '  t1["Read the Rust book"]:::task',
  '  d1{"Do I need a PhD?"}:::decision',
  '  t2["Apply to scholarships"]:::task',
  '  t1 --> d1',
  '  d1 -- yes --> t2',
].join('\n')

/** The "Copy for AI" prompt: the format, the closed kind lists, and the existing rows an AI may reference. */
export function roadmapPrompt(areas: Row[], projects: Row[], goals: Row[], tasks: Row[]): string {
  const list = (rows: Row[]) => rows.map((row) => `- ${row.title} (id ${row.id})`).join('\n') || '- none'
  return [
    'Write an Orbit roadmap as code. An Orbit roadmap is a graph of steps: tasks, goals, milestones, decisions with labelled branches, notes and headings, placed in optional bands (time periods).',
    'Reply with a single fenced code block and nothing else. Use exactly this format, version %%orbit:v1:',
    '',
    '```',
    PROMPT_EXAMPLE,
    '```',
    '',
    'Rules:',
    '- Node keys and band keys are identifiers (letters, digits, underscores). Each node line `%%orbit node <key> {...}` is followed by its shape line `<key>["Title"]:::<kind>` (decision nodes use `<key>{"Title"}:::decision`).',
    '- Node kinds, closed list: task, project, goal, milestone, decision, note, text, heading. Omit any other type.',
    '- Edge kinds, closed list: then, blocks, depends_on, decides. Use `then` when unsure.',
    '- A decision has one edge per branch; set "taken":true on at most one of its outgoing edges, only when it is already known.',
    '- "ref" must be the id of an existing row listed below, or left out. Never invent ids, and do not repeat a row that already exists.',
    '- Bands are optional time periods; "start" and "end" are YYYY-MM-DD dates. Put a node in a band with "band_id":"<band key>".',
    '- x and y are optional grid positions; leave them out if unsure.',
    '',
    'Areas (use as parent=<id> on the roadmap line):',
    list(areas),
    '',
    'Projects:',
    list(projects),
    '',
    'Goals:',
    list(goals),
    '',
    'Open tasks:',
    list(tasks),
  ].join('\n')
}
