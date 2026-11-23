// The guide order and labels live here; source files remain in the repository.
export default [
  {
    section: "Start here",
    slug: "start-here",
    order: 1,
    items: [
      { id: "README.md", slug: "what-is-orbit", title: "What Orbit is", summary: "Tasks, goals and routines you host yourself." },
      { id: "docs/first-ten-minutes.md", title: "Your first ten minutes", summary: "Go from nothing to a running server with one task synced to your laptop." },
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
      { id: "docs/sync-latency.md", title: "Sync latency baseline", summary: "Measured sync delay between Orbit clients, and how to re-run the harness." },
      { id: "docs/adr/README.md", title: "Decisions", summary: "Index of the architecture decision records that explain Orbit's choices." },
      { id: "docs/adr/0001-stack.md", title: "ADR 0001: Server-authoritative sync", summary: "Why Orbit uses a Rust core and server-authoritative sync." },
    ],
  },
  {
    section: "Contributing",
    slug: "contributing",
    order: 5,
    items: [
      { id: "docs/releasing.md", title: "Releasing Orbit", summary: "Release steps for maintainers." },
    ],
  },
];
