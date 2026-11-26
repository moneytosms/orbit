<script lang="ts">
  import Coffee from '@lucide/svelte/icons/coffee'
  import Code from '@lucide/svelte/icons/code'
  import ExternalLink from '@lucide/svelte/icons/external-link'
  import Globe from '@lucide/svelte/icons/globe'
  import Heart from '@lucide/svelte/icons/heart'
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
  const links = [
    { href: ABOUT.portfolio, label: 'Portfolio', icon: 'globe' },
    { href: ABOUT.github, label: 'GitHub profile', icon: 'github' },
    { href: ABOUT.repo, label: 'Orbit on GitHub', icon: 'code' },
    { href: ABOUT.coffee, label: 'Buy me a coffee', icon: 'coffee' },
    { href: ABOUT.kofi, label: 'Support on Ko-fi', icon: 'heart' },
  ]
  const startTour = () => window.dispatchEvent(new CustomEvent('orbit-tour'))
</script>

<section id="about" class="panel scroll-mt-4">
  <h2 class="panel-title">About</h2>

  <div class="flex flex-wrap items-baseline gap-x-3 gap-y-1">
    <span class="text-lg font-semibold">{ABOUT.name}</span>
    <span class="text-sm text-muted">Version {version}</span>
  </div>
  <p class="mt-1 text-sm text-muted">By {ABOUT.author}</p>

  <!-- Quiet icon links: each has a name for screen readers and a tooltip. -->
  <nav class="mt-3 flex items-center gap-0.5" aria-label="Links">
    {#each links as l (l.href)}
      <a
        href={l.href}
        target="_blank"
        rel="noreferrer"
        title={l.label}
        aria-label={l.label}
        class="grid size-9 place-items-center rounded-md text-muted transition-colors hover:bg-hover hover:text-text focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-accent-line"
      >
        {#if l.icon === 'github'}
          <svg viewBox="0 0 16 16" width="17" height="17" fill="currentColor" aria-hidden="true">
            <path
              d="M8 0C3.58 0 0 3.58 0 8c0 3.54 2.29 6.53 5.47 7.59.4.07.55-.17.55-.38 0-.19-.01-.82-.01-1.49-2.01.37-2.53-.49-2.69-.94-.09-.23-.48-.94-.82-1.13-.28-.15-.68-.52-.01-.53.63-.01 1.08.58 1.23.82.72 1.21 1.87.87 2.33.66.07-.52.28-.87.51-1.07-1.78-.2-3.64-.89-3.64-3.95 0-.87.31-1.59.82-2.15-.08-.2-.36-1.02.08-2.12 0 0 .67-.21 2.2.82a7.5 7.5 0 0 1 4 0c1.53-1.04 2.2-.82 2.2-.82.44 1.1.16 1.92.08 2.12.51.56.82 1.27.82 2.15 0 3.07-1.87 3.75-3.65 3.95.29.25.54.73.54 1.48 0 1.07-.01 1.93-.01 2.2 0 .21.15.46.55.38A8.01 8.01 0 0 0 16 8c0-4.42-3.58-8-8-8Z"
            />
          </svg>
        {:else if l.icon === 'globe'}
          <Globe size={17} aria-hidden="true" />
        {:else if l.icon === 'code'}
          <Code size={17} aria-hidden="true" />
        {:else if l.icon === 'coffee'}
          <Coffee size={17} aria-hidden="true" />
        {:else}
          <Heart size={17} aria-hidden="true" />
        {/if}
      </a>
    {/each}
  </nav>

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
