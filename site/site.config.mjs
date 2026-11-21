// The one file you edit per project. Everything else reads from here.
// Placeholders are filled by the skill's scripts/init.sh from `git remote`.
const owner = "moneytosms";
const repoName = "orbit";

export default {
  name: "Orbit",
  tagline: "A planner you host yourself.",
  description: "Orbit is a self-hostable personal planner: tasks, goals and routines synced to your own small Rust server, with web, Linux, Android and CLI clients that work offline.",
  owner,
  repoName,
  branch: "main",
  repoUrl: `https://github.com/${owner}/${repoName}`,
  // Project Pages live at https://OWNER.github.io/REPO/. For a custom domain
  // set site to https://docs.example.com and base to "".
  site: `https://${owner}.github.io`,
  base: `/${repoName}`,
  // Shown in the footer. State real constraints (synthetic data, alpha, etc.).
  footnote: "Orbit is AGPL-3.0. Android is smoke-tested on an emulator and one real phone (Android 16); wider device testing is still pending.",
  // Markdown rendered in place under /guide (repo-relative globs, "!" excludes).
  // No copies: a doc fixed in the repo is fixed on the site at the next build.
  guide: ["docs/*.md", "README.md"],
  guideRoot: "README.md", // becomes /guide/readme; docs/README.md becomes /guide/
};
