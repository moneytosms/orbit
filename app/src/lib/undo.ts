// A change the shell can offer to take back: App.svelte shows it as a toast with an Undo button.
export type Restorable = { text: string; restore: () => Promise<void> }

const listeners = new Set<(r: Restorable) => void>()

/** Calls `listener` for each change that can be undone; returns the unsubscribe. */
export const onRestorable = (listener: (r: Restorable) => void) => {
  listeners.add(listener)
  return () => listeners.delete(listener)
}

/** Offers `r` to the shell as an undoable change. */
export const pushRestore = (r: Restorable) => {
  for (const listener of listeners) listener(r)
}
