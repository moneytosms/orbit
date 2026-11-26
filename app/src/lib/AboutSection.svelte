<script lang="ts">
  import ExternalLink from '@lucide/svelte/icons/external-link'
  import ChevronRight from '@lucide/svelte/icons/chevron-right'
  import Keyboard from '@lucide/svelte/icons/keyboard'
  import Terminal from '@lucide/svelte/icons/terminal'
  import { Button } from '$lib/components/ui/button'
  import UsageGuide from '$lib/UsageGuide.svelte'
  import { ui } from '$lib/actions'
  import { ABOUT } from '$lib/about'

  const version = __ORBIT_APP_VERSION__
  const CLI_DOCS = 'https://github.com/moneytosms/orbit/blob/main/docs/cli.md'
  const LINK =
    'flex min-h-10 items-center gap-2 rounded-md px-1 text-left text-text hover:bg-hover focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent-line'
  const startTour = () => window.dispatchEvent(new CustomEvent('orbit-tour'))
</script>

<section id="about" class="panel scroll-mt-4">
  <h2 class="panel-title">About</h2>

  <div class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
    <span class="text-lg font-semibold">{ABOUT.name}</span>
    <span class="text-sm text-muted">Version {version}</span>
  </div>
  <p class="mt-1 text-sm text-muted">By {ABOUT.author}</p>

  <ul class="mt-4 flex flex-col gap-0.5">
    <li>
      <a class={LINK} href={ABOUT.github} target="_blank" rel="noreferrer">
        GitHub profile
        <ExternalLink size={14} class="shrink-0 text-icon" aria-hidden="true" />
      </a>
    </li>
    <li>
      <a class={LINK} href={ABOUT.repo} target="_blank" rel="noreferrer">
        Source repository
        <ExternalLink size={14} class="shrink-0 text-icon" aria-hidden="true" />
      </a>
    </li>
    {#if ABOUT.portfolio}
      <li>
        <a class={LINK} href={ABOUT.portfolio} target="_blank" rel="noreferrer">
          Portfolio
          <ExternalLink size={14} class="shrink-0 text-icon" aria-hidden="true" />
        </a>
      </li>
    {/if}
  </ul>

  <h3 class="mt-5 mb-1 text-[0.7812rem] font-medium tracking-wide text-muted uppercase">Guides</h3>
  <ul class="flex flex-col">
    <li class="border-b border-line last:border-0">
      <details class="group">
        <summary class="{LINK} cursor-pointer list-none [&::-webkit-details-marker]:hidden">
          <ChevronRight size={16} class="shrink-0 text-icon transition-transform group-open:rotate-90" />
          Quick add syntax
        </summary>
        <div class="pt-1 pb-3">
          <UsageGuide />
        </div>
      </details>
    </li>
    <li class="border-b border-line last:border-0">
      <button type="button" class="{LINK} w-full" onclick={() => ui.openHelp()}>
        <Keyboard size={16} class="shrink-0 text-icon" aria-hidden="true" />
        Keyboard shortcuts
      </button>
    </li>
    <li class="border-b border-line last:border-0">
      <a class={LINK} href={CLI_DOCS} target="_blank" rel="noreferrer">
        <Terminal size={16} class="shrink-0 text-icon" aria-hidden="true" />
        Command line
        <ExternalLink size={14} class="shrink-0 text-icon" aria-hidden="true" />
      </a>
    </li>
  </ul>

  <div class="mt-4 flex flex-wrap items-center justify-between gap-3 border-t border-line pt-4">
    <div>
      <div class="font-medium">Tour</div>
      <div class="text-[0.7812rem] text-muted">Replay the short walkthrough of Orbit.</div>
    </div>
    <Button size="sm" onclick={startTour}>Replay the tour</Button>
  </div>
</section>
