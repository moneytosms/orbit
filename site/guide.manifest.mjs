// The guide order and labels live here; source files remain in the repository.
export default [
  {
    section: "Start here",
    slug: "start-here",
    order: 1,
    items: [
      { id: "README.md", slug: "what-is-orbit", title: "What Orbit is", summary: "Tasks, goals and routines you host yourself." },
      { id: "docs/install.md", title: "Install", summary: "Set up Orbit on the web, Linux or Android." },
    ],
  },
  {
    section: "Use it",
    slug: "use-it",
    order: 2,
    items: [
      { id: "docs/cli-install.md", title: "Install and sign in to the CLI", summary: "Install Orbit CLI and connect it to your server." },
      { id: "docs/cli.md", title: "CLI reference", summary: "Commands, options, JSON output and exit codes." },
    ],
  },
  {
    section: "Run it",
    slug: "run-it",
    order: 3,
    items: [
      { id: "docs/deploy.md", title: "Deploy Orbit", summary: "Run the server and connect your devices." },
      { id: "docs/cloudflare-tunnel.md", title: "Cloudflare Tunnel", summary: "Expose Orbit securely through a Cloudflare Tunnel." },
      { id: "docs/android-checklist.md", title: "Android device checklist", summary: "Check device behavior and known Android limitations." },
      { id: "docs/linux-desktop.md", title: "Linux desktop app", summary: "Configure the desktop app, hotkeys and autostart." },
    ],
  },
  {
    section: "Reference",
    slug: "reference",
    order: 4,
    items: [
      { id: "docs/scope.md", title: "Scope", summary: "Orbit's current product scope and deferred work." },
      { id: "docs/adr/0001-stack.md", title: "ADR 0001: Server-authoritative sync", summary: "Why Orbit uses a Rust core and server-authoritative sync." },
    ],
  },
  {
    section: "Contributing",
    slug: "contributing",
    order: 5,
    items: [
      { id: "docs/releasing.md", title: "Releasing Orbit", summary: "Release steps for maintainers." },
      { id: "docs/agents/domain.md", title: "Domain documentation", summary: "Where Orbit's domain terms and decisions live." },
      { id: "docs/agents/issue-tracker.md", title: "Issue tracker conventions", summary: "How maintainers use GitHub Issues." },
      { id: "docs/agents/triage-labels.md", title: "Triage labels", summary: "The labels used to route Orbit issues." },
    ],
  },
];
