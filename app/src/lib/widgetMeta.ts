// What the widget gallery shows for each tile kind: an icon and one line of description.
// Add an entry here whenever a new kind is added to KINDS or PAGE_KINDS in dashboard.ts.
import type { Component } from 'svelte'
import CalendarDays from '@lucide/svelte/icons/calendar-days'
import Calendar from '@lucide/svelte/icons/calendar'
import ChartColumn from '@lucide/svelte/icons/chart-column'
import FileText from '@lucide/svelte/icons/file-text'
import Flame from '@lucide/svelte/icons/flame'
import Folder from '@lucide/svelte/icons/folder'
import Gauge from '@lucide/svelte/icons/gauge'
import LayoutList from '@lucide/svelte/icons/layout-list'
import ListChecks from '@lucide/svelte/icons/list-checks'
import Map from '@lucide/svelte/icons/map'
import Repeat from '@lucide/svelte/icons/repeat'
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
  view: { icon: LayoutList, desc: 'A saved view, as a list.' },
}

export const metaOf = (kind: string): WidgetMeta => META[kind] ?? { icon: LayoutList, desc: '' }
