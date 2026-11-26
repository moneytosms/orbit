<script lang="ts">
  import { GLYPHS, TIER_COLOR, metricOf, starPoints, tierKey } from '$lib/badgeIcons'

  let { id, tier, size = 'size-12' }: { id: string; tier: string; size?: string } = $props()
  const t = $derived(tierKey(tier))
  const c = $derived(TIER_COLOR[t] ?? TIER_COLOR.meteorite)
  const glyph = $derived(GLYPHS[metricOf(id)] ?? GLYPHS.stardust)
</script>

<svg viewBox="0 0 64 64" class="{size} shrink-0" aria-hidden="true">
  {#if t === 'comet'}
    <path d="M34 26L6 6l16 30z" fill={c} opacity="0.45" />
    <path d="M38 22L16 4l10 28z" fill={c} opacity="0.3" />
    <circle cx="40" cy="38" r="15" fill={c} />
  {:else if t === 'planet'}
    <circle cx="32" cy="32" r="18" fill={c} />
    <ellipse
      cx="32"
      cy="34"
      rx="29"
      ry="8"
      fill="none"
      stroke={c}
      stroke-width="3"
      opacity="0.5"
      transform="rotate(-20 32 34)"
    />
  {:else if t === 'star'}
    <polygon points={starPoints(8, 30, 17)} fill={c} />
  {:else if t === 'blackhole'}
    <ellipse
      cx="32"
      cy="32"
      rx="29"
      ry="10"
      fill="none"
      style="stroke: var(--line-strong, #D9D4CC)"
      stroke-width="3"
      opacity="0.8"
      transform="rotate(-18 32 32)"
    />
    <circle cx="32" cy="32" r="19" fill={c} />
    <circle
      cx="32"
      cy="32"
      r="19"
      fill="none"
      style="stroke: var(--line-strong, #D9D4CC)"
      stroke-width="1.5"
      opacity="0.5"
    />
  {:else}
    <path d="M12 32L21 13l19-3 12 13-2 20-14 11-17-4-7-9z" fill={c} />
    <circle cx="25" cy="25" r="3.2" fill="black" opacity="0.18" />
    <circle cx="43" cy="42" r="4" fill="black" opacity="0.18" />
    <path d="M4 18l9 5M3 30l7 2" stroke={c} stroke-width="2.5" stroke-linecap="round" opacity="0.5" />
  {/if}
  <g
    transform={t === 'comet' ? 'translate(28 26) scale(0.9)' : 'translate(20 20)'}
    fill="none"
    stroke="white"
    stroke-width="2.4"
    stroke-linecap="round"
    stroke-linejoin="round"
  >
    <!-- eslint-disable-next-line svelte/no-at-html-tags -->
    {@html glyph}
  </g>
</svg>
