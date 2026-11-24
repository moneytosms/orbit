// Repo-relative markdown path <-> guide route. Shared by the content loader
// and the remark link rewriter so the two can never disagree.
import cfg from "../../site.config.mjs";
import manifest, { redirects } from "../../guide.manifest.mjs";

export const REPO_URL = cfg.repoUrl;
export const BASE = cfg.base;
export const RAW_URL = `https://raw.githubusercontent.com/${cfg.owner}/${cfg.repoName}/${cfg.branch}`;
export const GUIDE_PATTERNS = cfg.guide;
export const GUIDE_INTERNAL_PATTERNS = cfg.guideInternal;

const ROUTES = new Map(manifest.flatMap((section) => section.items.map((item) => [item.id, item.slug ?? slugFromPath(item.id)])));

function slugFromPath(repoPath) {
  if (repoPath === "docs/README.md") return "index";
  if (repoPath === cfg.guideRoot) return "readme";
  return repoPath.replace(/\.md$/, "").replace(/^docs\//, "").replace(/\/README$/, "").toLowerCase();
}

/** "docs/adr/0001-x.md" -> "adr/0001-x"; "docs/foo/README.md" -> "foo". */
export function slugFor(repoPath) {
  return ROUTES.get(repoPath) ?? slugFromPath(repoPath);
}

export const GUIDE_REDIRECTS = Object.fromEntries([
  ...Object.entries(redirects).map(([from, to]) => [`/guide/${from}`, `${cfg.base}/guide/${to}`]),
  ...manifest.flatMap((section) => section.items.flatMap((item) => {
    const previous = slugFromPath(item.id);
    const current = slugFor(item.id);
    return previous === current ? [] : [[`/guide/${previous}`, `${cfg.base}/guide/${current}`]];
  })),
]);

/** Mirrors GUIDE_PATTERNS for a single path (supports "dir/**\/*.md", exact files and "!" excludes). */
export function isGuidePath(repoPath) {
  if (!repoPath.endsWith(".md")) return false;
  const match = (glob) => {
    const re = new RegExp("^" + glob.replace(/[.+^${}()|[\]\\]/g, "\\$&").replace(/\*\*\//g, "(?:.*/)?").replace(/\*/g, "[^/]*") + "$");
    return re.test(repoPath);
  };
  const include = GUIDE_PATTERNS.filter((p) => !p.startsWith("!")).some(match);
  const exclude = GUIDE_PATTERNS.filter((p) => p.startsWith("!")).some((p) => match(p.slice(1)));
  return include && !exclude;
}

export function isInternalGuidePath(repoPath) {
  const match = (glob) => {
    const re = new RegExp("^" + glob.replace(/[.+^${}()|[\]\\]/g, "\\$&").replace(/\*\*\//g, "(?:.*/)?").replace(/\*/g, "[^/]*") + "$");
    return re.test(repoPath);
  };
  return GUIDE_INTERNAL_PATTERNS.some((pattern) => match(pattern));
}
