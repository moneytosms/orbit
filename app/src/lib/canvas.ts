import type { PhaseSummary } from './sync'

export const FRAME_W = 310
export const FRAME_GAP = 40
export const FRAME_MIN_H = 120
export const STEP_W = 278
export const STEP_H = 56
export const STEP_PITCH = 68
export const STEP_TOP = 84
export const FRAME_PAD = 16
export const ZOOM_MIN = 0.4
export const ZOOM_MAX = 2

export type Item = PhaseSummary['items'][number]
export type FrameBox = {
  id: string
  x: number
  y: number
  w: number
  h: number
  state: 'done' | 'active' | 'locked' | 'todo'
}
export type StepBox = { id: string; phaseId: string; x: number; y: number; w: number; h: number; item: Item }
export type Link = { from: string; to: string; locked: boolean }
export type Layout = { frames: FrameBox[]; steps: StepBox[]; links: Link[]; gates: { from: string; to: string }[] }

export const clampZoom = (z: number) => Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, z))

export function phaseState(p: PhaseSummary): FrameBox['state'] {
  return p.completed ? 'done' : p.locked ? 'locked' : p.current ? 'active' : 'todo'
}

/** Frames left to right by order, each stacking its items; links are the directed arrows between placed steps. */
export function layout(phases: PhaseSummary[]): Layout {
  const frames: FrameBox[] = []
  const steps: StepBox[] = []
  const lockedPhase = new Map<string, boolean>()
  const at = new Map<string, StepBox>()
  phases.forEach((p, i) => {
    const x = i * (FRAME_W + FRAME_GAP)
    frames.push({
      id: p.id,
      x,
      y: 0,
      w: FRAME_W,
      h: Math.max(FRAME_MIN_H, STEP_TOP + p.items.length * STEP_PITCH + FRAME_PAD - (p.items.length ? 12 : 0)),
      state: phaseState(p),
    })
    p.items.forEach((item, j) => {
      const box = {
        id: item.id,
        phaseId: p.id,
        x: x + FRAME_PAD,
        y: STEP_TOP + j * STEP_PITCH,
        w: STEP_W,
        h: STEP_H,
        item,
      }
      steps.push(box)
      at.set(item.id, box)
      lockedPhase.set(item.id, p.locked)
    })
  })
  const links: Link[] = steps.flatMap((s) =>
    s.item.links
      .filter((to) => at.has(to) && to !== s.id)
      .map((to) => ({ from: s.id, to, locked: !!lockedPhase.get(to) })),
  )
  const where = new Map(steps.map((s) => [s.id, s]))
  const gates = phases.flatMap((p) => {
    if (!p.gate_id || !where.has(p.gate_id)) return []
    return [{ from: p.gate_id, to: p.id }]
  })
  return { frames, steps, links, gates }
}

/** Cubic path between two step boxes: side to side across frames, top to bottom within one column. */
export function linkPath(a: StepBox, b: StepBox): string {
  if (Math.abs(a.x - b.x) < 60) {
    const down = b.y > a.y
    const x1 = a.x + a.w / 2
    const y1 = down ? a.y + a.h : a.y
    const y2 = down ? b.y - 2 : b.y + b.h + 2
    const k = down ? 18 : -18
    return `M${x1} ${y1}C${x1} ${y1 + k} ${x1} ${y2 - k} ${x1} ${y2}`
  }
  const right = b.x > a.x
  const x1 = right ? a.x + a.w : a.x
  const x2 = right ? b.x - 2 : b.x + b.w + 2
  const y1 = a.y + a.h / 2
  const y2 = b.y + b.h / 2
  const k = Math.max(40, Math.abs(x2 - x1) / 2) * (right ? 1 : -1)
  return `M${x1} ${y1}C${x1 + k} ${y1} ${x2 - k} ${y2} ${x2} ${y2}`
}

/** Dashed gate line from the Milestone step to the left edge of the gated frame. */
export function gatePath(m: StepBox, f: FrameBox): string {
  const x1 = m.x + m.w
  const y1 = m.y + m.h / 2
  const x2 = f.x
  const y2 = f.y + 28
  const k = Math.max(40, Math.abs(x2 - x1) / 2) * (x2 >= x1 ? 1 : -1)
  return `M${x1} ${y1}C${x1 + k} ${y1} ${x2 - k} ${y2} ${x2} ${y2}`
}

/** Frame that a world point falls in, by x then y; null when between frames. */
export function frameAt(frames: FrameBox[], x: number, y: number): FrameBox | null {
  return frames.find((f) => x >= f.x && x <= f.x + f.w && y >= f.y && y <= f.y + f.h) ?? null
}

/** Zoom about a screen point so the world point under it stays put. */
export function zoomAt(
  view: { x: number; y: number; z: number },
  factor: number,
  px: number,
  py: number,
): { x: number; y: number; z: number } {
  const z = clampZoom(view.z * factor)
  const k = z / view.z
  return { x: px - (px - view.x) * k, y: py - (py - view.y) * k, z }
}

/** Pan and zoom that fit every frame inside a viewport with a margin. */
export function fit(
  frames: FrameBox[],
  width: number,
  height: number,
  margin = 48,
): { x: number; y: number; z: number } {
  if (!frames.length) return { x: 40, y: 76, z: 0.9 }
  const w = Math.max(...frames.map((f) => f.x + f.w))
  const h = Math.max(...frames.map((f) => f.y + f.h))
  const z = clampZoom(Math.min((width - margin * 2) / w, (height - margin * 2 - 40) / h, 1))
  return { x: (width - w * z) / 2, y: margin + 40, z }
}
