// The quick-add cheat sheet shown in Settings and mirrored in docs/quick-add-reference.md.
// Every example is checked against the real parser in usageGuide.test.ts, so the guide cannot drift.
export type Expect = {
  title?: string
  /** `YYYY-MM-DD` or `YYYY-MM-DD HH:MM`, with now = 2026-11-19 10:00 (a Thursday). */
  due?: string
  deadline?: string
  duration?: number
  priority?: number
  place?: string
  goal?: string
  page?: string
  labels?: string[]
  recurrence?: string
  reminders?: number
}
export type Example = { text: string; means: string; expect: Expect }
export type Group = { id: string; title: string; intro: string; examples: Example[] }

export const GUIDE_NOW = '2026-11-19T10:00'
/** Names the guide's examples resolve against; the test uses the same ones. */
export const GUIDE_KNOWN = {
  areas: ['Home', 'Work'],
  goals: ['Get fit'],
  pages: ['Trip notes'],
  labels: ['errand', 'deep work'],
}

export const guide: Group[] = [
  {
    id: 'dates',
    title: 'Dates and times',
    intro: 'Type a date the way you would say it. The words are removed from the title and shown as a chip.',
    examples: [
      { text: 'pay rent tomorrow', means: 'Due tomorrow.', expect: { title: 'pay rent', due: '2026-11-20' } },
      {
        text: 'call mom fri 6pm',
        means: 'Next Friday at 18:00.',
        expect: { title: 'call mom', due: '2026-11-20 18:00' },
      },
      {
        text: 'dentist next fri',
        means: '`next` means the one in next week.',
        expect: { title: 'dentist', due: '2026-11-27' },
      },
      {
        text: 'gym tomorrow morning',
        means: 'Morning is 09:00, afternoon 14:00, evening 18:00, night 21:00.',
        expect: { title: 'gym', due: '2026-11-20 09:00' },
      },
      { text: 'review tonight', means: 'Tonight is 20:00.', expect: { title: 'review', due: '2026-11-19 20:00' } },
      { text: 'dinner this weekend', means: 'The coming Saturday.', expect: { title: 'dinner', due: '2026-11-21' } },
      {
        text: 'taxes end of month',
        means: 'Also: next month, next year, mid jan.',
        expect: { title: 'taxes', due: '2026-11-30' },
      },
      {
        text: 'plan the 27th',
        means: 'The next 27th. `on the 27th` works too.',
        expect: { title: 'plan', due: '2026-11-27' },
      },
      {
        text: 'pay on 27/1',
        means: 'Numbers read day/month: 27 January.',
        expect: { title: 'pay', due: '2027-01-27' },
      },
    ],
  },
  {
    id: 'relative',
    title: 'Relative times',
    intro: 'Counted from right now, down to the minute.',
    examples: [
      {
        text: 'check oven in 20 min',
        means: 'Due today at the time 20 minutes from now.',
        expect: { title: 'check oven', due: '2026-11-19 10:20' },
      },
      {
        text: 'stand up in half an hour',
        means: 'Also: in 2 hours, in 1h30m, 90 minutes from now.',
        expect: { title: 'stand up', due: '2026-11-19 10:30' },
      },
    ],
  },
  {
    id: 'deadline',
    title: 'Deadline and duration',
    intro: 'A deadline is the day it must be done by; the due date is when you plan to work on it.',
    examples: [
      {
        text: 'report by friday',
        means: '`by <date>` sets the deadline, not the due date.',
        expect: { title: 'report', deadline: '2026-11-20' },
      },
      {
        text: 'ship it {dec 5}',
        means: 'Curly braces do the same.',
        expect: { title: 'ship it', deadline: '2026-12-05' },
      },
      {
        text: 'write essay for 2h',
        means: '`for` sets how long it takes: 30 min, 1h15m, half an hour.',
        expect: { title: 'write essay', duration: 120 },
      },
      {
        text: 'meeting 3-4pm tomorrow',
        means: 'A time range sets the time and the duration.',
        expect: { title: 'meeting', due: '2026-11-20 15:00', duration: 60 },
      },
    ],
  },
  {
    id: 'where',
    title: 'Where it lives',
    intro: 'Names are matched without caring about case. An unknown name stays in the title.',
    examples: [
      {
        text: 'book flights %trip notes @work p1',
        means: '`@` Area or Project, `%` Page, `p1` to `p4` priority.',
        expect: { title: 'book flights', priority: 1, place: 'Work', page: 'Trip notes' },
      },
      {
        text: 'buy milk #errand',
        means: '`#` adds a label. Add as many as you like.',
        expect: { title: 'buy milk', labels: ['errand'] },
      },
      { text: 'run 5k $get fit', means: '`$` links a Goal.', expect: { title: 'run 5k', goal: 'Get fit' } },
      { text: 'issue \\#1', means: 'A backslash keeps a symbol as plain text.', expect: { title: 'issue #1' } },
    ],
  },
  {
    id: 'repeat',
    title: 'Repeating',
    intro: '`every` repeats from the due date. `every!` repeats from the day you complete it.',
    examples: [
      {
        text: 'water plants every 3 days',
        means: 'Also: every day, every week, every month, every quarter.',
        expect: { title: 'water plants', due: '2026-11-19', recurrence: '3 days' },
      },
      {
        text: 'standup every weekday at 9am',
        means: 'A time after the rule applies to every occurrence.',
        expect: { title: 'standup', due: '2026-11-19 09:00', recurrence: 'weekday' },
      },
      {
        text: 'trash every other fri',
        means: 'Every second Friday.',
        expect: { title: 'trash', due: '2026-11-20', recurrence: '2 weeks on fri' },
      },
      {
        text: 'rent every last fri',
        means: 'Also: every 2nd wed, every first workday.',
        expect: { title: 'rent', due: '2026-11-27', recurrence: 'last fri' },
      },
      {
        text: 'gym every mon,wed until dec 31',
        means: 'Stop with `until <date>`; start with `starting <date>`.',
        expect: { title: 'gym', due: '2026-11-23', recurrence: 'mon,wed end 2026-12-31' },
      },
    ],
  },
  {
    id: 'reminders',
    title: 'Reminders',
    intro: '`!` adds a reminder: before the due time, or at a fixed time.',
    examples: [
      {
        text: 'call mom tomorrow 6pm !30m',
        means: '30 minutes before. Also !1h, !1d.',
        expect: { title: 'call mom', due: '2026-11-20 18:00', reminders: 1 },
      },
      {
        text: 'take pill !later',
        means: 'In 3 hours. Also !morning, !evening, !tonight, !fri 9am, !in 10 minutes.',
        expect: { title: 'take pill', reminders: 1 },
      },
    ],
  },
]
