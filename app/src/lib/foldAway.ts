// The fold-away: a row that is about to be moved to Trash or archived collapses in place before its data changes,
// so the list closes the gap smoothly. Skipped when motion is off, or when the system asks for less.
import { motionAllowed } from './completionFx'

const FOLD_MS = 240

/** Collapses `el` to nothing (height, opacity, padding and margin) and resolves when the fold has finished. */
export async function foldAway(el: Element | null): Promise<void> {
  if (!el || !(el instanceof HTMLElement) || !motionAllowed() || typeof el.animate !== 'function') return
  const style = getComputedStyle(el)
  const height = el.getBoundingClientRect().height
  el.style.overflow = 'hidden'
  const animation = el.animate(
    [
      {
        height: `${height}px`,
        opacity: 1,
        paddingTop: style.paddingTop,
        paddingBottom: style.paddingBottom,
        marginTop: style.marginTop,
        marginBottom: style.marginBottom,
      },
      {
        height: '0px',
        opacity: 0,
        paddingTop: '0px',
        paddingBottom: '0px',
        marginTop: '0px',
        marginBottom: '0px',
      },
    ],
    { duration: FOLD_MS, easing: 'ease', fill: 'forwards' },
  )
  try {
    await animation.finished
  } catch {
    /* cancelled by the browser; the row is removed by the data change either way */
  }
}

/** The list row for `id`: a task row first, then any element marked `data-row-id`. */
export function findRow(id: string): Element | null {
  const key = CSS.escape(id)
  return document.querySelector(`[data-task-row="${key}"]`) ?? document.querySelector(`[data-row-id="${key}"]`)
}
