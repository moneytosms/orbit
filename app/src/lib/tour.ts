// The first-run tour: what each step points at, and where its card sits. Pure so it can be tested.
import { get, writable } from 'svelte/store'

/** True while the spotlight tour is on screen. Set by Tour.svelte; other openers (Ctrl+K, search) check it. */
export const tourOpen = writable(false)

/** Whether a new overlay such as the command palette may open now: not while the tour is showing. */
export function canOpenOverlay(isTourOpen: boolean = get(tourOpen)): boolean {
  return !isTourOpen
}
export type Step = {
  title: string
  body: string
  /** A CSS selector for the element to spotlight; without one (or without a visible match) the card is centred. */
  target?: string
  /** Skip the step entirely when its target is not on screen. */
  optional?: boolean
}

export const STEPS: Step[] = [
  { title: 'Welcome to Orbit', body: 'Tasks, goals and routines in one calm place. This takes about thirty seconds.' },
  {
    title: 'Add a task',
    body: 'Press Q or tap here any time. Type it the way you would say it: “call mom tomorrow 6pm !30m p2”.',
    target: '[data-tour="add"]',
  },
  {
    title: 'Find your way around',
    body: 'Home, Inbox, Upcoming and your Areas live here. Drag things onto them to move them.',
    target: '[data-tour="nav"]',
  },
  {
    title: 'Make a page yours',
    body: 'Open the ⋯ menu on Home, an Area or a Project and choose Edit layout to rearrange it.',
    target: '[data-tour="page-menu"]',
    optional: true,
  },
  {
    title: 'Settings, archive and trash',
    body: 'Tap your name for your profile. The arrow next to it opens Settings, Archive & Trash, and this tour again lives in Settings under About.',
    target: '[data-tour="account"]',
  },
]

export const TOUR_KEY = 'orbit.tour.done'

export type Box = { left: number; top: number; width: number; height: number }

/** The first element for `selector` that is actually on screen (a phone hides the desktop sidebar), as a box. */
export function visibleBox(selector: string, root: ParentNode = document): Box | null {
  for (const el of root.querySelectorAll<HTMLElement>(selector)) {
    const r = el.getBoundingClientRect()
    if (r.width > 0 && r.height > 0) return { left: r.left, top: r.top, width: r.width, height: r.height }
  }
  return null
}

/** Where to put a card of `card` size next to `box` inside `view`: below if it fits, else above, kept on screen. */
export function cardPosition(
  box: Box | null,
  card: { width: number; height: number },
  view: { width: number; height: number },
  gap = 12,
): { left: number; top: number } {
  const margin = 12
  if (!box) return { left: (view.width - card.width) / 2, top: Math.max(margin, (view.height - card.height) / 2) }
  const clampX = (x: number) => Math.min(Math.max(margin, x), view.width - card.width - margin)
  const below = box.top + box.height + gap
  const top = below + card.height + margin <= view.height ? below : Math.max(margin, box.top - gap - card.height)
  return { left: clampX(box.left + box.width / 2 - card.width / 2), top }
}
