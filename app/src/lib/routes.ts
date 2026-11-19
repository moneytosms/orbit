import CalendarDays from '@lucide/svelte/icons/calendar-days'
import CircleUser from '@lucide/svelte/icons/circle-user'
import House from '@lucide/svelte/icons/house'
import Inbox from '@lucide/svelte/icons/inbox'
import Map from '@lucide/svelte/icons/map'
import Repeat from '@lucide/svelte/icons/repeat'
import Settings from '@lucide/svelte/icons/settings'
import Target from '@lucide/svelte/icons/target'
import Trash2 from '@lucide/svelte/icons/trash-2'

/** Every screen in the shell, by hash route. Inbox, Settings and Goals are built; the rest show a stub. */
export const routes = {
  home: { label: 'Home', icon: House },
  inbox: { label: 'Inbox', icon: Inbox },
  upcoming: { label: 'Upcoming', icon: CalendarDays },
  goals: { label: 'Goals', icon: Target },
  roadmaps: { label: 'Roadmaps', icon: Map },
  routines: { label: 'Routines', icon: Repeat },
  profile: { label: 'Profile', icon: CircleUser },
  trash: { label: 'Trash', icon: Trash2 },
  settings: { label: 'Settings', icon: Settings },
}
export type Route = keyof typeof routes

export const readRoute = (): Route => {
  const r = location.hash.replace(/^#\/?/, '')
  return r in routes ? (r as Route) : 'inbox'
}

/** The screen on screen: a fixed route, or a Project, Area, Page or View by id. */
export type Screen = { name: Route } | { name: 'project' | 'area' | 'page' | 'view'; id: string }

export const readScreen = (): Screen => {
  const m = location.hash.match(/^#\/(project|area|page|view)\/([^/]+)$/)
  return m
    ? { name: m[1] as 'project' | 'area' | 'page' | 'view', id: decodeURIComponent(m[2]) }
    : { name: readRoute() }
}
