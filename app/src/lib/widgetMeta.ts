// What the widget gallery shows for each tile kind: an icon and one line of description.
// Add an entry here whenever a new kind is added to KINDS or PAGE_KINDS in dashboard.ts.
import type { Component } from 'svelte'
import CalendarCheck from '@lucide/svelte/icons/calendar-check'
import CalendarDays from '@lucide/svelte/icons/calendar-days'
import AlarmClock from '@lucide/svelte/icons/alarm-clock'
import Calendar from '@lucide/svelte/icons/calendar'
import ChartPie from '@lucide/svelte/icons/chart-pie'
import ChartColumn from '@lucide/svelte/icons/chart-column'
import CircleCheckBig from '@lucide/svelte/icons/circle-check-big'
import FileText from '@lucide/svelte/icons/file-text'
import Flame from '@lucide/svelte/icons/flame'
import Folder from '@lucide/svelte/icons/folder'
import Gauge from '@lucide/svelte/icons/gauge'
import LayoutList from '@lucide/svelte/icons/layout-list'
import ListChecks from '@lucide/svelte/icons/list-checks'
import Map from '@lucide/svelte/icons/map'
import Plus from '@lucide/svelte/icons/plus'
import Repeat from '@lucide/svelte/icons/repeat'
import Tag from '@lucide/svelte/icons/tag'
import Target from '@lucide/svelte/icons/target'

export type WidgetMeta = { icon: Component<{ size?: number; class?: string }>; desc: string }

export const META: Record<string, WidgetMeta> = {
  glance: { icon: Gauge, desc: 'Due today, overdue, routines and goals in four numbers.' },
  next: { icon: ListChecks, desc: 'The few things that most need you right now.' },
  routines: { icon: Repeat, desc: "Today's routines to tick off." },
  upcoming: { icon: CalendarDays, desc: 'What is due in the coming days.' },
  goals: { icon: Target, desc: 'Progress on your goals.' },
  streaks: { icon: Flame, desc: 'Your routine streaks.' },
  calendar: { icon: Calendar, desc: 'Events from your connected calendars.' },
  stats: { icon: ChartColumn, desc: 'Open, done this week, overdue and goals here.' },
  tasks: { icon: ListChecks, desc: 'The open tasks on this page.' },
  pages: { icon: FileText, desc: 'Notes and docs that live here.' },
  projects: { icon: Folder, desc: 'The projects in this area.' },
  roadmaps: { icon: Map, desc: 'Roadmaps that plan work in phases.' },
  today: { icon: CalendarCheck, desc: 'Open tasks due today.' },
  overdue: { icon: AlarmClock, desc: 'Late tasks, with one tap to move them to today.' },
  done: { icon: CircleCheckBig, desc: 'The last eight tasks you finished.' },
  progress: { icon: ChartPie, desc: 'Tasks done this week against what is due, as a ring.' },
  quickadd: { icon: Plus, desc: 'Type a task and press Enter to add it.' },
  labels: { icon: Tag, desc: 'Your most used labels with task counts.' },
  view: { icon: LayoutList, desc: 'A saved view, as a list.' },
}

export const metaOf = (kind: string): WidgetMeta => META[kind] ?? { icon: LayoutList, desc: '' }
