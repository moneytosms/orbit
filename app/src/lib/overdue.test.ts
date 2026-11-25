import { describe, expect, it } from 'vitest'
import {
  addDays,
  newDue,
  overdueLabel,
  overdueTasks,
  planMoves,
  POSTPONE,
  targetDate,
  weekday,
  type Project,
} from './overdue'
import type { Task } from './sync'

// 2026-11-25 is a Wednesday.
const TODAY = '2026-11-25'

const task = (over: Partial<Task> & { id: string }): Task =>
  ({
    kind: 'task',
    parent_id: 'inbox',
    title: over.id,
    completed_at: null,
    order: 'a',
    description: '',
    links: [],
    due: null,
    deadline: null,
    duration_min: null,
    priority: 4,
    label_ids: [],
    recurrence: null,
    goal_id: null,
    created_at: null,
    trashed_at: null,
    reminders: [],
    ...over,
  }) as Task

describe('overdueTasks', () => {
  it('keeps open, untrashed Tasks dated before today, earliest first', () => {
    const tasks = [
      task({ id: 'today', due: { date: TODAY } }),
      task({ id: 'future', due: { date: '2026-12-01' } }),
      task({ id: 'undated', due: null }),
      task({ id: 'done', due: { date: '2026-11-01' }, completed_at: 5 }),
      task({ id: 'trashed', due: { date: '2026-11-01' }, trashed_at: 5 }),
      task({ id: 'late', due: { date: '2026-11-20' } }),
      task({ id: 'early', due: { date: '2026-11-02' } }),
    ]
    expect(overdueTasks(tasks, TODAY).map((t) => t.id)).toEqual(['early', 'late'])
  })

  it('breaks date ties by title', () => {
    const tasks = [
      task({ id: 'b', title: 'Beta', due: { date: '2026-11-10' } }),
      task({ id: 'a', title: 'Alpha', due: { date: '2026-11-10' } }),
    ]
    expect(overdueTasks(tasks, TODAY).map((t) => t.id)).toEqual(['a', 'b'])
  })

  it('counts a timed Task on an earlier day as overdue, and one due today as not', () => {
    const tasks = [
      task({ id: 'yesterday-9am', due: { date: '2026-11-24', time: '09:00' } }),
      task({ id: 'today-9am', due: { date: TODAY, time: '09:00' } }),
    ]
    expect(overdueTasks(tasks, TODAY).map((t) => t.id)).toEqual(['yesterday-9am'])
  })
})

describe('overdueLabel', () => {
  it('reads the count', () => {
    expect(overdueLabel(1)).toBe('1 overdue')
    expect(overdueLabel(4)).toBe('4 overdue')
  })
})

describe('date helpers', () => {
  it('addDays crosses months and years', () => {
    expect(addDays('2026-11-30', 1)).toBe('2026-12-01')
    expect(addDays('2026-12-31', 1)).toBe('2027-01-01')
    expect(addDays('2026-03-01', -1)).toBe('2026-02-28')
  })

  it('weekday uses 0 for Sunday', () => {
    expect(weekday('2026-11-25')).toBe(3)
    expect(weekday('2026-11-29')).toBe(0)
  })
})

describe('targetDate', () => {
  it('today and tomorrow', () => {
    expect(targetDate('today', TODAY)).toBe(TODAY)
    expect(targetDate('tomorrow', TODAY)).toBe('2026-11-26')
  })

  it('next week starts on the chosen weekday after today', () => {
    // Wednesday 25 Nov: Monday is 30 Nov, Sunday is 29 Nov.
    expect(targetDate('nextWeek', TODAY, { weekStart: 1 })).toBe('2026-11-30')
    expect(targetDate('nextWeek', TODAY, { weekStart: 0 })).toBe('2026-11-29')
    expect(targetDate('nextWeek', TODAY)).toBe('2026-11-30')
  })

  it('next week on a Monday is the Monday after', () => {
    expect(targetDate('nextWeek', '2026-11-30', { weekStart: 1 })).toBe('2026-12-07')
  })

  it('this weekend is the coming Saturday, or today at the weekend', () => {
    expect(targetDate('weekend', TODAY)).toBe('2026-11-28')
    expect(targetDate('weekend', '2026-11-28')).toBe('2026-11-28')
    expect(targetDate('weekend', '2026-11-29')).toBe('2026-11-29')
    expect(targetDate('weekend', '2026-11-30')).toBe('2026-12-05')
  })

  it('+1 day and +1 week count from the later of the due date and today', () => {
    expect(targetDate('plus1d', TODAY)).toBe('2026-11-26')
    expect(targetDate('plus1w', TODAY)).toBe('2026-12-02')
    // A Task due in the future is pushed from its due date.
    expect(targetDate('plus1d', TODAY, { from: '2026-12-10' })).toBe('2026-12-11')
    // An overdue Task is pushed from today, not from its old due date.
    expect(targetDate('plus1w', TODAY, { from: '2026-11-01' })).toBe('2026-12-02')
  })

  it('none has no date', () => {
    expect(targetDate('none', TODAY)).toBeNull()
  })

  it('every postpone choice is listed once', () => {
    expect(POSTPONE.map(([c]) => c)).toEqual(['tomorrow', 'nextWeek', 'weekend', 'plus1d', 'plus1w', 'none'])
  })
})

describe('newDue', () => {
  it('a plain Task takes the date and keeps its time of day', () => {
    const t = task({ id: 'x', due: { date: '2026-11-01', time: '14:30' } })
    expect(newDue(t, TODAY)).toEqual({ date: TODAY, time: '14:30' })
  })

  it('a plain Task without a time takes only the date', () => {
    const t = task({ id: 'x', due: { date: '2026-11-01' } })
    expect(newDue(t, TODAY)).toEqual({ date: TODAY })
  })

  it('clearing gives null', () => {
    expect(newDue(task({ id: 'x', due: { date: '2026-11-01' } }), null)).toBeNull()
  })

  it('a recurring Task moves to its first occurrence on or after the target', () => {
    const t = task({
      id: 'r',
      due: { date: '2026-11-11', time: '08:00' },
      recurrence: { rule: 'every week', from_completion: false },
    })
    // The series has an occurrence today, so it lands there, keeping the series time.
    const onToday: Project = () => [
      { date: '2026-11-18', time: '08:00' },
      { date: '2026-11-25', time: '08:00' },
      { date: '2026-12-02', time: '08:00' },
    ]
    expect(newDue(t, TODAY, onToday)).toEqual({ date: TODAY, time: '08:00' })
    // Here the series skips today, so the next occurrence after it is taken.
    const skipsToday: Project = () => [
      { date: '2026-11-18', time: '08:00' },
      { date: '2026-12-02', time: '08:00' },
    ]
    expect(newDue(t, TODAY, skipsToday)).toEqual({ date: '2026-12-02', time: '08:00' })
  })

  it('a recurring Task with no projection falls back to the plain move', () => {
    const t = task({ id: 'r', due: { date: '2026-11-11' }, recurrence: { rule: 'every day', from_completion: false } })
    expect(newDue(t, TODAY, () => [])).toEqual({ date: TODAY })
  })
})

describe('planMoves', () => {
  it('moves every overdue Task and skips the ones already on the target', () => {
    const a = task({ id: 'a', due: { date: '2026-11-10' } })
    const b = task({ id: 'b', due: { date: TODAY, time: '10:00' } })
    const c = task({ id: 'c', due: { date: '2026-11-20', time: '10:00' } })
    const moves = planMoves([a, b, c], TODAY)
    expect(moves).toEqual([
      { id: 'a', before: { date: '2026-11-10' }, after: { date: TODAY } },
      { id: 'c', before: { date: '2026-11-20', time: '10:00' }, after: { date: TODAY, time: '10:00' } },
    ])
  })

  it('a clear plans a null due for every dated Task', () => {
    const a = task({ id: 'a', due: { date: '2026-11-10' } })
    expect(planMoves([a], null)).toEqual([{ id: 'a', before: { date: '2026-11-10' }, after: null }])
  })

  it('plans nothing for Tasks already on the target', () => {
    expect(planMoves([task({ id: 'a', due: { date: TODAY } })], TODAY)).toEqual([])
  })

  it('the Move records the original due so an undo can restore it', () => {
    const a = task({ id: 'a', due: { date: '2026-11-10', time: '07:15' } })
    const [m] = planMoves([a], '2026-11-26')
    expect(m.before).toEqual({ date: '2026-11-10', time: '07:15' })
  })
})
