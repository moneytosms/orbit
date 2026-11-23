// Check the built GitHub Pages site for broken local links, images and anchors.
// Usage: node scripts/verify.mjs [dist-dir] [base-path]
// Defaults to site/dist and the base from site.config.mjs.
import { execFileSync } from "node:child_process";
import { existsSync, readFileSync, readdirSync, statSync } from "node:fs";
import { dirname, extname, join, relative, resolve, sep } from "node:path";
import { fileURLToPath } from "node:url";
import cfg from "../site.config.mjs";
import manifest from "../guide.manifest.mjs";
import { GUIDE_PATTERNS, GUIDE_REDIRECTS, isGuidePath, isInternalGuidePath, slugFor } from "../src/lib/slug.mjs";

const here = fileURLToPath(new URL("..", import.meta.url));
const dist = resolve(process.argv[2] ?? join(here, "dist"));
const repo = resolve(here, "..");
const base = `/${(process.argv[3] ?? cfg.base).split("/").filter(Boolean).join("/")}/`;
const failures = [];
const warnings = [];
let markdownLinks = 0;
let repositoryLinks = 0;

// Only git-tracked Markdown counts, so untracked local notes never fail the check.
const trackedMarkdown = execFileSync("git", ["-C", repo, "ls-files", "-z", "--", "*.md"], { encoding: "utf8" })
  .split("\0")
  .filter(Boolean)
  .map((path) => join(repo, path));

function walk(dir) {
  if (!existsSync(dir)) return [];
  return readdirSync(dir, { withFileTypes: true }).flatMap((entry) => {
    const path = join(dir, entry.name);
    return entry.isDirectory() ? walk(path) : [path];
  });
}

if (!existsSync(dist)) {
  console.error(`Build output not found: ${dist}`);
  process.exit(2);
}

const guideItems = manifest.flatMap((section) => section.items);
const guideIds = guideItems.map((item) => item.id);
const checkedTitles = [];
for (const [id, count] of [...new Set(guideIds)].map((id) => [id, guideIds.filter((candidate) => candidate === id).length])) {
  if (count !== 1) failures.push(`Guide manifest: ${id} appears ${count} times`);
}
const sourceMarkdown = trackedMarkdown
  .filter((path) => !relative(repo, path).split(sep).some((part) => [".git", "node_modules", "dist", ".astro"].includes(part)));
const guideMarkdown = sourceMarkdown.filter((path) => isGuidePath(relative(repo, path).split(sep).join("/")));
for (const file of sourceMarkdown) {
  const id = relative(repo, file).split(sep).join("/");
  if (id.startsWith("docs/") && !isGuidePath(id) && !isInternalGuidePath(id)) {
    failures.push(`Markdown under docs/ must be in the guide manifest or explicitly internal: ${id}`);
  }
}
for (const file of guideMarkdown) {
  const id = relative(repo, file).split(sep).join("/");
  const count = guideIds.filter((candidate) => candidate === id).length;
  if (count !== 1) failures.push(`Guide manifest: ${id} matched by guide patterns ${GUIDE_PATTERNS.join(", ")} but listed ${count} times`);
}
for (const item of guideItems) {
  const file = resolve(repo, item.id);
  if (!existsSync(file)) {
    failures.push(`Guide manifest: missing source ${item.id}`);
    continue;
  }
  if (!isGuidePath(item.id)) failures.push(`Guide manifest: ${item.id} is not matched by configured guide patterns`);
  const source = readFileSync(file, "utf8");
  failures.push(...markdownSourceFailures(item.id, source));
  const h1 = [...stripMarkdownCode(source).matchAll(/^\s{0,3}#\s+(.+?)\s*#*\s*$/gm)].map((match) => match[1].trim());
  if (h1.length > 1 || (h1.length === 0 && !item.title)) failures.push(`Guide manifest: ${item.id} needs exactly one H1 or a manifest title`);
  const title = (item.title ?? h1[0] ?? "").replace(/[`*_]/g, "").trim().toLowerCase();
  if (item.summary && item.summary.length > 160) failures.push(`Guide manifest: summary for ${item.id} exceeds 160 characters`);
  if (title) checkedTitles.push(title);
}
for (const title of new Set(checkedTitles)) if (checkedTitles.filter((candidate) => candidate === title).length > 1) failures.push(`Guide manifest: duplicate page title “${title}”`);
for (const [oldPath, destination] of Object.entries(GUIDE_REDIRECTS)) {
  const rel = `${oldPath.replace(/^\//, "")}/index.html`;
  const redirectFile = join(dist, rel);
  const target = destination;
  if (!existsSync(redirectFile)) failures.push(`Guide redirect missing output: ${oldPath}`);
  else if (!readFileSync(redirectFile, "utf8").includes(target)) failures.push(`Guide redirect ${oldPath} does not point to ${target}`);
}
for (const item of guideItems) {
  const slug = item.slug ?? slugFor(item.id);
  const routeFile = slug ? join(dist, "guide", ...slug.split("/"), "index.html") : join(dist, "guide", "index.html");
  const basedRouteFile = join(dist, base.replace(/^\//, "").replace(/\/$/, ""), "guide", ...slug.split("/"), "index.html");
  if (!existsSync(routeFile) && !existsSync(basedRouteFile)) failures.push(`Guide route missing output: /guide/${slug}`);
}
for (const section of manifest) {
  const sectionFile = join(dist, "guide", "sections", section.slug, "index.html");
  if (!existsSync(sectionFile)) failures.push(`Guide section route missing output: /guide/sections/${section.slug}`);
}
if (!existsSync(join(dist, "guide", "index.html"))) failures.push("Guide index route missing output: /guide/");
console.log(`Checked guide structure: ${manifest.length} sections, ${guideItems.length} pages, ${guideMarkdown.length} source files.`);

const htmlFiles = walk(dist).filter((path) => extname(path) === ".html");
const pages = new Map();
for (const file of htmlFiles) {
  const rel = relative(dist, file).split(sep).join("/");
  const route = rel.endsWith("/index.html")
    ? `${base}${rel.slice(0, -"index.html".length)}`
    : `${base}${rel}`;
  const key = route.replace(/\/$/, "") || base.slice(0, -1);
  pages.set(key === base.slice(0, -1) ? base : key, readFileSync(file, "utf8"));
}

function decode(value) {
  try { return decodeURIComponent(value); } catch { return value; }
}

function stripMarkdownCode(source) {
  const lines = source.split(/\r?\n/);
  let fence = null;
  const kept = [];
  for (const line of lines) {
    const marker = line.match(/^\s{0,3}(`{3,}|~{3,})/);
    if (marker) {
      if (!fence) fence = marker[1][0];
      else if (marker[1][0] === fence) fence = null;
      kept.push("");
    } else kept.push(fence ? "" : line);
  }
  return kept.join("\n").replace(/(`+)(.*?)\1/g, " ");
}

// Markdown source rules: fenced code needs a language, no placeholder text, no empty
// sections, sane heading levels, ADRs carry Status and Date. Fenced content is skipped.
function markdownSourceFailures(id, source) {
  const problems = [];
  let fence = null;
  let pending = null;
  let previous = null;
  let seenH2 = false;
  const lines = source.split(/\r?\n/);
  lines.forEach((line, index) => {
    const at = `${id}:${index + 1}`;
    const marker = line.match(/^\s{0,3}(`{3,}|~{3,})(.*)$/);
    if (fence) {
      pending = null;
      if (marker && marker[1][0] === fence.char && marker[1].length >= fence.length && marker[2].trim() === "") fence = null;
      return;
    }
    if (marker) {
      pending = null;
      fence = { char: marker[1][0], length: marker[1].length };
      if (marker[2].trim() === "") problems.push(`${at}: fenced code block has no language`);
      return;
    }
    if (/\bTODO\b|\bcoming soon\b|\blorem\b/i.test(line)) problems.push(`${at}: placeholder text (TODO, coming soon or lorem)`);
    if (!line.trim()) return;
    const heading = line.match(/^ {0,3}(#{1,6})(?=\s|$)\s*(.*)$/);
    if (!heading) {
      pending = null;
      return;
    }
    const level = heading[1].length;
    if (pending && level <= pending.level) problems.push(`${at}: empty section “${pending.text}”`);
    pending = { level, text: heading[2].trim() };
    if (previous !== null && level > previous + 1) problems.push(`${at}: heading jumps from H${previous} to H${level}`);
    if (level >= 3 && !seenH2) problems.push(`${at}: H${level} appears before the first H2`);
    if (level === 2) seenH2 = true;
    previous = level;
  });
  if (pending) problems.push(`${id}:${lines.length}: empty section “${pending.text}” at end of file`);
  if (/^docs\/adr\/[^/]+\.md$/.test(id) && id !== "docs/adr/README.md") {
    if (!/^Status:/m.test(source)) problems.push(`${id}: ADR has no Status: line`);
    if (!/^Date:/m.test(source)) problems.push(`${id}: ADR has no Date: line`);
  }
  return problems;
}

function markdownHeadings(source) {
  const result = new Set();
  const counts = new Map();
  for (const line of source.split(/\r?\n/)) {
    const match = line.match(/^ {0,3}#{1,6}\s+(.+?)\s*#*\s*$/);
    if (!match) continue;
    const plain = match[1]
      .replace(/<[^>]+>/g, "")
      .replace(/!?\[([^\]]*)\]\([^)]*\)/g, "$1")
      .replace(/[`*_~]/g, "")
      .replace(/&(?:amp|lt|gt|quot|#39);/g, "")
      .trim()
      .toLowerCase();
    const slug = plain.replace(/[^\p{L}\p{N}\-_\s]/gu, "").replace(/\s/g, "-");
    const n = counts.get(slug) ?? 0;
    counts.set(slug, n + 1);
    result.add(n ? `${slug}-${n}` : slug);
  }
  for (const match of source.matchAll(/\bid\s*=\s*["']([^"']+)["']/gi)) result.add(match[1]);
  return result;
}

function markdownDestination(value) {
  let dest = value.trim();
  if (dest.startsWith("<") && dest.endsWith(">")) dest = dest.slice(1, -1);
  return dest.replace(/\\([\\`*_{}\[\]()#+.!|>])/g, "$1");
}

function markdownTarget(sourceFile, destination) {
  const raw = markdownDestination(destination);
  if (!raw || /^(?:[a-z][a-z\d+.-]*:|\/\/)/i.test(raw)) return null;
  const [pathPart, hashPart = ""] = raw.split(/#(.*)/s);
  const targetPath = pathPart ? resolve(dirname(sourceFile), decode(pathPart)) : sourceFile;
  const choices = [
    targetPath,
    `${targetPath}.md`,
    join(targetPath, "README.md"),
  ];
  const file = choices.find((candidate) => existsSync(candidate) && statSync(candidate).isFile());
  if (!file) return { raw, targetPath, hash: hashPart, file: null, directory: existsSync(targetPath) && statSync(targetPath).isDirectory() };
  return { raw, targetPath, hash: hashPart, file };
}

function checkMarkdownLink(sourceFile, destination, sourceText) {
  const raw = markdownDestination(destination);
  if (!raw) return;
  const gh = raw.match(new RegExp(`^${cfg.repoUrl.replace(/[.\/]/g, "\\$&")}/(?:blob|tree)/[^/]+/(.*?)(?:[?#].*)?$`, "i"));
  if (gh) {
    repositoryLinks++;
    const repoPath = decode(gh[1]).replace(/\/$/, "");
    const target = resolve(repo, repoPath);
    if (repoPath && !existsSync(target)) failures.push(`${relative(repo, sourceFile)}: broken repository link ${raw}`);
    return;
  }
  const target = markdownTarget(sourceFile, raw);
  if (!target) return;
  markdownLinks++;
  if (!target.file) {
    if (target.directory && !target.hash) return;
    failures.push(`${relative(repo, sourceFile)}: missing Markdown target ${raw}`);
    return;
  }
  if (target.hash) {
    const anchors = markdownHeadings(target.file === sourceFile ? sourceText : readFileSync(target.file, "utf8"));
    const anchor = decode(target.hash);
    if (!anchors.has(anchor)) failures.push(`${relative(repo, sourceFile)}: missing Markdown anchor ${raw}`);
  }
}

function localFile(urlPath) {
  let pathname = decode(urlPath);
  if (pathname === base.slice(0, -1)) pathname = base;
  if (pathname.startsWith(base)) pathname = pathname.slice(base.length);
  else if (pathname.startsWith("/")) return null;
  else return null;
  pathname = pathname.replace(/^\/+/, "");
  const candidates = pathname.endsWith("/")
    ? [join(dist, pathname, "index.html")]
    : [join(dist, pathname), join(dist, `${pathname}.html`), join(dist, pathname, "index.html")];
  return candidates.find((candidate) => existsSync(candidate) && statSync(candidate).isFile()) ?? null;
}

function attributes(html, name) {
  const out = [];
  const escaped = name.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const re = new RegExp(`\\b${escaped}\\s*=\\s*(?:"([^"]*)"|'([^']*)'|([^\\s>]+))`, "gi");
  for (const match of html.matchAll(re)) out.push(match[1] ?? match[2] ?? match[3] ?? "");
  return out;
}

function ids(html) {
  const set = new Set();
  for (const value of [...attributes(html, "id"), ...attributes(html, "name")]) set.add(value);
  return set;
}

let localRefs = 0;
let imageRefs = 0;
let anchorRefs = 0;
let shotFigures = 0;
const referencedPages = new Map([...pages.keys()].map((route) => [route, ids(pages.get(route))]));

for (const [route, html] of pages) {
  // Ignore strings inside inline JavaScript; only HTML attributes are links.
  const markup = html.replace(/<script\b[^>]*>[\s\S]*?<\/script>/gi, (block) => block.slice(0, block.indexOf(">") + 1));
  const refs = [
    ...attributes(markup, "href").map((value) => ["href", value]),
    ...attributes(markup, "src").map((value) => ["src", value]),
  ];
  for (const raw of attributes(markup, "srcset")) {
    for (const candidate of raw.split(",")) {
      const value = candidate.trim().split(/\s+/)[0];
      if (value) refs.push(["srcset", value]);
    }
  }

  for (const [kind, raw] of refs) {
    const value = raw.trim();
    if (!value || /^(?:data:|mailto:|tel:|javascript:|blob:)/i.test(value)) continue;
    let parsed;
    try { parsed = new URL(value, `https://local.invalid${route}`); }
    catch { failures.push(`${route}: invalid ${kind} ${value}`); continue; }
    if (parsed.origin !== "https://local.invalid") continue;
    localRefs++;
    const target = localFile(parsed.pathname);
    if (!target) {
      failures.push(`${route}: missing local ${kind} ${value}`);
      continue;
    }
    if (kind === "src" || kind === "srcset") imageRefs++;
    if (parsed.hash) {
      const targetRoute = parsed.pathname.replace(/\/$/, "") || base.slice(0, -1);
      const normalizedTarget = targetRoute === base.slice(0, -1) ? base : targetRoute;
      const targetHtml = readFileSync(target, "utf8");
      const targetIds = referencedPages.get(normalizedTarget) ?? ids(targetHtml);
      const fragment = decode(parsed.hash.slice(1));
      anchorRefs++;
      if (!targetIds.has(fragment)) failures.push(`${route}: missing anchor ${value}`);
    }
  }

  if (/Screenshot pending:/i.test(html) || /class=["'][^"']*shot__missing/i.test(html)) {
    failures.push(`${route}: screenshot placeholder found`);
  }
  if (/\bTODO\b|lorem ipsum/i.test(markup.replace(/<[^>]+>/g, " "))) failures.push(`${route}: leftover TODO or lorem ipsum`);
  shotFigures += (markup.match(/class="[^"]*shot--(?:phone|browser)/g) ?? []).length;

  const visible = markup
    .replace(/<(script|style|pre|code|svg)[\s\S]*?<\/\1>/gi, " ")
    .replace(/<!--([\s\S]*?)-->/g, " ")
    .replace(/<[^>]*>/g, " ")
    .replace(/&(?:nbsp|#160);/gi, " ");
  for (const match of visible.matchAll(/(?<![A-Za-z0-9])\d+(?:patients?|users?|entries|entry|tests?|roles?|permissions?|locales?|days?|minutes?|hours?|providers?|rows?|services?|specs?|migrations?|commits?|issues?)\b/gi)) {
    warnings.push(`${route}: missing space before “${match[0]}”`);
  }
}

const markdownFiles = trackedMarkdown.filter((path) => {
  const rel = relative(repo, path).split(sep);
  return extname(path).toLowerCase() === ".md" && !rel.some((part) => [".git", ".claude", "node_modules", ".venv", "dist", ".astro"].includes(part));
});
for (const file of markdownFiles) {
  const source = readFileSync(file, "utf8");
  const clean = stripMarkdownCode(source);
  const links = [];
  for (const match of clean.matchAll(/!?\[[^\]]*\]\(\s*(<[^>]+>|(?:\\.|[^\s)])+)(?:\s+[^)]*)?\)/g)) links.push(match[1]);
  for (const match of clean.matchAll(/^\s{0,3}\[[^\]]+\]:\s*(<[^>]+>|\S+)/gm)) links.push(match[1]);
  for (const attr of ["href", "src"]) links.push(...attributes(clean, attr));
  for (const link of links) checkMarkdownLink(file, link, source);
  for (const match of clean.matchAll(/(?<![A-Za-z0-9])\d+(?:patients?|users?|entries|entry|tests?|roles?|permissions?|locales?|days?|minutes?|hours?|providers?|rows?|services?|specs?|migrations?|commits?|issues?)\b/gi)) {
    warnings.push(`${relative(repo, file)}: missing space before “${match[0]}”`);
  }
}

const screenshotPngs = walk(join(here, "src/assets/screens")).filter((path) => extname(path).toLowerCase() === ".png");
const screenshotSources = [
  ...walk(join(here, "src/pages")).filter((path) => extname(path) === ".astro"),
  ...walk(join(here, "src/sections")).filter((path) => extname(path) === ".astro"),
];
const referencedScreens = new Set();
const missingScreens = new Set();
for (const file of screenshotSources) {
  const source = readFileSync(file, "utf8");
  if (extname(file) === ".astro") {
    for (const component of source.matchAll(/<Screenshot\b[\s\S]*?>/g)) {
      for (const match of component[0].matchAll(/\bname\s*=\s*["']([a-z0-9-]+)["']/gi)) referencedScreens.add(match[1]);
    }
  } else {
    for (const match of source.matchAll(/\bscreen\s*:\s*["']([a-z0-9-]+)["']/gi)) referencedScreens.add(match[1]);
  }
}
const sourceScreenNames = new Set(screenshotPngs.map((path) => path.slice(join(here, "src/assets/screens/").length, -4)));
for (const name of referencedScreens) if (!sourceScreenNames.has(name)) missingScreens.add(name);
for (const name of missingScreens) failures.push(`Screenshot reference has no source image: ${name}`);
const unusedScreens = [...sourceScreenNames].filter((name) => !referencedScreens.has(name)).sort();

console.log(`Checked ${htmlFiles.length} HTML files, ${localRefs} local references, ${imageRefs} image references, ${anchorRefs} anchors.`);
console.log(`Checked ${markdownLinks} Markdown links and ${repositoryLinks} GitHub repository paths across ${markdownFiles.length} source documents.`);
console.log(`Found ${screenshotPngs.length} source app screenshots and ${shotFigures} rendered screenshot figures.`);
console.log(`Found ${referencedScreens.size} named screenshot references; ${unusedScreens.length} source images are unused${unusedScreens.length ? ` (${unusedScreens.join(", ")})` : ""}.`);
for (const warning of warnings) console.warn(`WARN ${warning}`);
for (const failure of failures) console.error(`FAIL ${failure}`);
if (failures.length) process.exitCode = 1;
else console.log("Static output links, images, anchors and screenshot placeholders look good.");
