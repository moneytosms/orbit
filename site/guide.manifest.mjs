// The guide order and labels live here; source files remain in the repository.
// kind "guide" pages must follow the page shape (audience line, Before you start,
// Verify it worked, Troubleshooting, Next); "reference" pages are exempt from it.
// `redirects` maps retired guide slugs to the page that absorbed them.
export const redirects = {
  "android-checklist": "android",
};

/** @type {{ section: string, slug: string, order: number, items: { id: string, title: string, summary: string, slug?: string, kind?: "guide" | "reference" }[] }[]} */
export default [
  {
    section: "Start here",
    slug: "start-here",
    order: 1,
    items: [
      { id: "README.md", slug: "what-is-orbit", kind: "reference", title: "What Orbit is", summary: "Tasks, goals and routines you host yourself, with web, desktop, Android and CLI clients." },
      { id: "docs/install.md", title: "Install", summary: "Put a client on each device: web, desktop, Android and CLI, plus updating and removing them." },
      { id: "docs/first-ten-minutes.md", title: "Your first ten minutes", summary: "Go from nothing to a running server with one task synced to your laptop." },
    ],
  },
  {
    section: "Use it",
    slug: "use-it",
    order: 2,
    items: [
      { id: "docs/tasks.md", title: "Quick add and tasks", summary: "Capture a task in one line: dates, priorities, labels, places, goals, recurrence and reminders." },
      { id: "docs/views.md", title: "Views and filters", summary: "Save the list you want to see, with visual or typed filters, grouping and layouts." },
      { id: "docs/pages.md", title: "Pages", summary: "Write plans and notes next to your tasks, with links, backlinks and foldable sections." },
      { id: "docs/routines.md", title: "Routines and goals", summary: "Track habits and outcomes without overdue pile-ups, and see when a goal is behind pace." },
      { id: "docs/roadmaps.md", title: "Roadmaps", summary: "Plan a goal as a graph of tasks and decisions, and copy it to or from an AI as text." },
      { id: "docs/cli-install.md", title: "CLI: install and sign in", summary: "Install the orbit command, connect it to your server, update it and use it from AI tools." },
      { id: "docs/cli.md", title: "CLI: reference", kind: "reference", summary: "Every command, option, JSON output shape and exit code, generated from the CLI." },
      { id: "docs/linux-desktop.md", title: "Desktop app", summary: "Install the Linux tray app, set the quick capture hotkey and start it at login." },
      { id: "docs/android.md", title: "Android app", summary: "Install the APK, set up widgets and reminders, and run the on-device checklist." },
    ],
  },
  {
    section: "Run it",
    slug: "run-it",
    order: 3,
    items: [
      { id: "docs/deploy.md", title: "Deploy Orbit", summary: "Run the server, put HTTPS in front of it, keep it running and back it up." },
      { id: "docs/cloudflare-tunnel.md", title: "Deploy: Cloudflare Tunnel", summary: "Expose a VPS-hosted Orbit through a Cloudflare Tunnel instead of opening ports." },
    ],
  },
  {
    section: "Reference",
    slug: "reference",
    order: 4,
    items: [
      { id: "docs/scope.md", title: "Scope", kind: "reference", summary: "Orbit's product scope, principles and what is deferred." },
      { id: "GLOSSARY.md", slug: "glossary", kind: "reference", title: "Glossary", summary: "The exact meaning of Orbit's words: Area, Section, Phase, Routine, Check-in and more." },
      { id: "docs/sync-latency.md", title: "Sync latency baseline", kind: "reference", summary: "Measured sync delay between Orbit clients, and how to re-run the harness." },
      { id: "docs/adr/README.md", title: "Decisions", kind: "reference", summary: "Index of the architecture decision records that explain Orbit's choices." },
      { id: "docs/adr/0001-stack.md", title: "ADR 0001: Server-authoritative sync", kind: "reference", summary: "Why Orbit uses a Rust core and server-authoritative sync." },
      { id: "docs/adr/0002-roadmap-graph.md", title: "ADR 0002: Roadmap graph canvas", kind: "reference", summary: "Why roadmaps are free-positioned graphs with bands, rendered with DOM nodes and SVG edges." },
    ],
  },
  {
    section: "Contributing",
    slug: "contributing",
    order: 5,
    items: [
      { id: "docs/architecture.md", title: "Architecture tour", summary: "Which crate owns what, and how a change travels from the UI to the server and back." },
      { id: "docs/contributing.md", title: "Contributing", summary: "Set up, run the gate with just check, and follow the repository's conventions." },
      { id: "docs/releasing.md", title: "Releasing Orbit", summary: "Cut a release, with the client and protocol policy, secrets and packaging." },
    ],
  },
];
