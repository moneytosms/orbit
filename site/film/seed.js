// Seeds the Orbit web demo with a believable, made-up planner for the film and the site's screenshots.
// Runs inside the page (Vite dev server with VITE_DEMO=1), through the app's own sync module, so every
// row is created by the same commands the real app sends. No real user data is involved.
export async function seedOrbit() {
  const s = await import('/src/lib/sync.ts')
  const realNow = Date.now
  const DAY = 86400000
  const day = (offset) => {
    const d = new Date(realNow() + offset * DAY)
    return `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, '0')}-${String(d.getDate()).padStart(2, '0')}`
  }

  for (const name of ['Career', 'College', 'Learning', 'Home']) await s.createArea(name)
  let areas = await s.tree()
  const area = (n) => areas.find((a) => a.area.name === n).area.id
  const [career, college, learning, home] = ['Career', 'College', 'Learning', 'Home'].map(area)

  await s.createProject(career, 'Internship hunt')
  await s.createProject(career, 'Portfolio site')
  await s.createProject(college, 'Thesis')
  await s.createProject(learning, 'Rust book')
  await s.createProject(home, 'Spring cleaning')
  areas = await s.tree()
  const project = (n) => areas.flatMap((a) => a.projects).find((p) => p.name === n).id

  const labels = {}
  for (const l of ['deep-work', 'errand', 'call']) labels[l] = await s.createLabel(l)

  const t = async (title, parent, due, priority, label) => {
    const id = await s.createTask(title, parent)
    if (due !== null && due !== undefined) await s.setDue(id, day(due))
    if (priority) await s.updateTask(id, { priority })
    if (label) await s.setLabels(id, [labels[label]])
    return id
  }

  // Inbox
  await t('Call mum about the weekend', undefined, 0, 3, 'call')
  await t('Pick up the dry cleaning', undefined, 1, undefined, 'errand')
  await t('Book the dentist', undefined, 4)

  // Career
  const hunt = project('Internship hunt')
  for (const n of ['To apply', 'Applied', 'Interviewing']) await s.createSection(hunt, n)
  const sec = Object.fromEntries((await s.sectionsOf(hunt)).map((x) => [x.name, x.id]))
  await t('Apply to two internships', sec['To apply'], 0, 1, 'deep-work')
  await t('Ask Priya for a referral', sec['To apply'], 1, 2)
  await t('Research salary bands', sec['To apply'], 6)
  await t('Tailor CV for Lumen Labs', sec['To apply'], 2, 3)
  await t('Follow up with Northwind recruiter', sec['Applied'], -1, 2, 'call')
  await t('Atlas Robotics: take-home test', sec['Applied'], 4, 2, 'deep-work')
  await t('Prep for the systems interview', sec['Interviewing'], 3, 1, 'deep-work')
  await t('Thank-you note to Helio team', sec['Interviewing'], 1)
  const folio = project('Portfolio site')
  await t('Write the Orbit case study', folio, 2, 2, 'deep-work')
  await t('Refresh the headshot', folio, 8)
  await t('Fix the contact form', folio, 5, 3)

  // College
  const thesis = project('Thesis')
  await t('Send chapter 2 to supervisor', thesis, 0, 1, 'deep-work')
  await t('Book a lab slot for the experiment', thesis, -1, 2)
  await t('Clean the survey data', thesis, 2, 2)
  await t('Draft the methods section', thesis, 4, 3, 'deep-work')
  await t('Print the ethics form', thesis, 9, undefined, 'errand')

  // Learning
  const rust = project('Rust book')
  await t('Finish the ownership chapter', rust, 0, 3, 'deep-work')
  await t('Do the lifetimes exercises', rust, 3)
  await t('Build the mini grep project', rust, 7)

  // Home
  const clean = project('Spring cleaning')
  await t('Pay rent', home, 0, 1)
  await t('Order a birthday gift for Sam', home, 2, 2, 'errand')
  await t('Sort the garage', clean, 5)
  await t('Donate old clothes', clean, 6, undefined, 'errand')
  await t('Renew the bike insurance', home, 12)

  // Goals: a target goal, a milestone goal and sub-goals so progress rolls up.
  const books = await s.createGoal(learning, 'Read 12 books this year', {
    progress: { kind: 'target', target: 12, entries: [] },
    target_date: day(36),
  })
  await s.addEntry(books, 9, 'Nine down')
  const offer = await s.createGoal(career, 'Land a summer internship', { target_date: day(70) })
  const m1 = await s.createMilestone(offer, 'Portfolio live', day(-20))
  const m2 = await s.createMilestone(offer, '20 applications sent', day(-4))
  await s.createMilestone(offer, 'Three interviews', day(21))
  await s.createMilestone(offer, 'Signed offer', day(70))
  await s.setMilestoneDone(m1, true)
  await s.setMilestoneDone(m2, true)
  const grad = await s.createGoal(college, 'Submit the thesis', { target_date: day(120) })
  for (const [title, d, done] of [
    ['Proposal approved', -60, true],
    ['Literature review', -25, true],
    ['Experiment run', 14, false],
    ['Final draft', 100, false],
  ]) {
    const m = await s.createMilestone(grad, title, day(d))
    if (done) await s.setMilestoneDone(m, true)
  }
  const fit = await s.createGoal(home, 'Run a half marathon', {
    progress: { kind: 'target', target: 21, entries: [] },
    target_date: day(90),
  })
  await s.addEntry(fit, 14, 'Long run: 14 km')

  // Routines with history: shift the clock back so they start weeks ago, then check in day by day.
  const routine = async (parent, name, icon, back, time, misses = []) => {
    Date.now = () => realNow() - back * DAY
    const id = await s.createRoutine(parent, { name, icon, schedule: { kind: 'daily' }, length_days: null, time, goal_id: null })
    Date.now = realNow
    for (let i = back; i >= 1; i--) if (!misses.includes(i)) await s.checkIn(id, day(-i), 'done')
    return id
  }
  const run = await routine(home, 'Morning run', '🏃', 63, '07:00', [41, 40, 22])
  await s.checkIn(run, day(0), 'done')
  await routine(learning, 'Read 20 pages', '📚', 48, '21:30', [30])
  await routine(learning, 'Spanish practice', '🗣️', 38, '19:00', [12])
  await routine(home, 'Meditate', '🧘', 26, '06:45')

  // Roadmap: ordered phases that gather goals, projects and tasks without moving them.
  const map = await s.createRoadmap(career, 'Graduate to first job')
  const p1 = await s.createPhase(map, 'Foundations', day(-90), day(-10))
  const p2 = await s.createPhase(map, 'Applications', day(-10), day(40))
  const p3 = await s.createPhase(map, 'Interviews', day(40), day(80))
  const p4 = await s.createPhase(map, 'Offer & move', day(80), day(130))
  await s.placeItem(map, folio, p1)
  await s.placeItem(map, m1, p1)
  await s.placeItem(map, hunt, p2)
  await s.placeItem(map, m2, p2)
  await s.placeItem(map, offer, p3)
  await s.placeItem(map, grad, p4)
  await s.setPhaseDone(p1, true)

  await s.setProfile({ display_name: 'Alex' })
  return { map, thesis, hunt, offer, books, home }
}
