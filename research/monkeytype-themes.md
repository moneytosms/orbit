# Monkeytype theme palettes: reuse in Orbit

## Question

Can Orbit (AGPL-3.0) reuse Monkeytype's theme palettes? Primary sources only: monkeytypegame/monkeytype at commit `dd28fa71fb92638f29b0fc083895d953bfa65a7d` (committer date 2026-11-18T19:45:41+01:00). Shallow clone, read-only. Repo scripts not executed.

## Short answer

Not settled by the repo. The repo is GPL-3.0 with no theme-specific licence, no attribution rule and no separate terms for theme files. Reuse would be governed by GPL-3.0 terms for the palette data; whether AGPL-3.0 Orbit may take GPL-3.0 material is a licence-compatibility question the repo does not answer. Needs a licence decision by the user, not a repo fact.

## License verdict

- Project licence: GPL-3.0 (not AGPL). `package.json` L5 and `frontend/package.json` L4 both say `"license": "GPL-3.0"`. `LICENSE` L1-2 is GNU GPL Version 3, 29 June 2007.
- Separate terms for theme files: not found. No THEMES/LICENSE/NOTICE/COPYING file under `docs/THEMES.md` or `frontend/static/themes/`. No licence or copyright header in `frontend/src/ts/constants/themes.ts`, `packages/schemas/src/themes.ts`, or any `frontend/static/themes/*.css` (searched for SPDX, copyright, licence, author, credit, "by").
- The repo does not say whether palettes (hex values) are copyrightable. Not found.

## Attribution

- No attribution requirement found in `LICENSE`, `README.md` (Credits section L64-68 names people for name suggestion and feedback, and contributors generally; no rule about reuse), `docs/THEMES.md`, or `docs/CONTRIBUTING.md` L53-64.
- Any attribution obligation would come only from GPL-3.0 itself (`LICENSE`), not from a Monkeytype-specific term. Not found beyond that.

## Theme definition location and format

- Name enum: `packages/schemas/src/themes.ts` L4 `ThemeNameSchema = z.enum([...])`, 187 string names (L5 onward). Lowercase, underscores (per `docs/THEMES.md` L16-27).
- Colour schema: `frontend/src/ts/constants/themes.ts` L4-9 (`hexColorSchema`, regex `#RGB`, `#RGBA`, `#RRGGBB`, `#RRGGBBAA`), L11-23 (`ThemeSchema`, zod object).
- Colour keys (all required hex except `hasCss`):
  - `bg`, `main`, `caret`, `sub`, `subAlt`, `text`, `error`, `errorExtra`, `colorfulError`, `colorfulErrorExtra`
  - `hasCss` (optional boolean): loads `frontend/static/themes/<name>.css`
- Map: `frontend/src/ts/constants/themes.ts` L26 `export const themes: Record<ThemeName, Theme> = {` (entries keyed by theme name, e.g. `"8008"` L27-38, `"9009"` L39-51 with `hasCss: true`).
- Sorted list: `frontend/src/ts/constants/themes.ts` L2326-2331 `ThemesList` (sorted by key).
- Keying/loading: `ThemeName` is the schema enum. Config fields `themeLight`, `themeDark`, `randomTheme`, `favThemes` at `packages/schemas/src/configs.ts` L497-500.
- Contributor process: `docs/THEMES.md` L29-47 (add to enum and map), L57-76 (optional CSS).
- Example data point: `frontend/src/ts/constants/themes.ts` L27-38 (`8008`: bg `#333a45`, caret `#f44c7f`, main `#f44c7f`, sub `#939eae`, subAlt `#2e343d`, text `#e9ecf0`, error `#da3333`, errorExtra `#791717`, colorfulError `#c5da33`, colorfulErrorExtra `#849224`).
- No licence or provenance field per theme. Not found.

## Theme count

- Full count, method: parsed `frontend/src/ts/constants/themes.ts` (keys of the `themes` object, regex on `  name: {` blocks) and `packages/schemas/src/themes.ts` (quoted names in the enum). Python 3 `-I` reading text only; no repo code run.
- Results:
  - `themes` object: 187 entries, 187 unique keys.
  - `ThemeNameSchema`: 187 names, 187 unique.
  - Set difference between the two: none.
  - Themes with `hasCss: true`: 52.
  - `frontend/static/themes/*.css`: 52 files.
- So the count is complete (not partial): 187 themes, 52 with custom CSS.

## Light/dark split

- No explicit light/dark field. `ThemeSchema` (`frontend/src/ts/constants/themes.ts` L11-23) has none. Names with `_light`/`_dark` suffixes exist (e.g. `packages/schemas/src/themes.ts` L20-21) but the code does not use names for classification.
- Code classifies by background lightness: `frontend/src/ts/utils/colors.ts` L226-229 `isColorLight` (`lgt >= 50`), L236-239 `isColorDark` (`lgt < 50`), where `lgt` comes from `hexToHSL` (L132-200) as `(max+min)/2 * 100`. Used for random light/dark (`frontend/src/ts/controllers/theme-controller.ts` L173-176), auto-switch (L203, L238-254), and body dark mode (`frontend/src/ts/components/core/Theme.tsx` L66).
- My split, computed from `bg` with the same rule (reimplemented from the source above; app code not run):
  - Light (lgt >= 50): 63
  - Dark (lgt < 50): 124
  - Borderline (48 to 52): none
- Light: 9009, beach, bingsu, blueberry_light, botanical, breeze, cafe, camping, cheesecake, creamsicle, darling, desert_oasis, dino, dmg, dollar, fleuriste, froyo, frozen_llama, fruit_chew, godspeed, gruvbox_light, hanok, iceberg_light, iv_clover, lavender, lil_dragon, lilac_mist, lime, macroblank, magic_girl, mexican, miami, milkshake, mizu, modern_dolch_light, modern_ink, mr_sleeves, ms_cupcakes, nord_light, olive, paper, pastel, peaches, pink_lemonade, rainbow_trail, repose_light, retro, rose_pine_dawn, serika, sewing_tin_light, shoko, slambook, snes, soaring_skies, solarized_light, strawberry, tangerine, taro, terrazzo, tiramisu, vaporwave, vesper_light, witch_girl.
- Caveat: the rule is on background only, so a name suffix can disagree (e.g. `serika` is light, `serika_dark` is dark; `miami` is light, `miami_nights` is dark).

## Popularity/readability signal

- Popularity (votes, usage counts, install counts): not found in the repo.
- Favourites: `packages/schemas/src/configs.ts` L500 `favThemes` is a per-user config field. No aggregate counts found. Not a repo-level signal.
- Readability: `docs/CONTRIBUTING.md` L58-62 sets submission criteria (unique, black or white text colour, clear and readable in flip test and colorful modes). This is a contribution rule, not a measured signal. No WCAG or contrast score found in theme files. Searched for `vote`, `popular`, `readab`, `contrast`, `wcag`, `likes`, `usage` in theme code.
- Stars are repo-wide, not theme-specific: not used.

## Unverified / not found

- Any licence for theme palette data separate from GPL-3.0: not found.
- Attribution requirement specific to themes: not found.
- Theme popularity, votes, usage stats: not found.
- Per-theme authorship or provenance: not found.
- GPL-3.0 / AGPL-3.0 compatibility for combining code: not checked; outside the Monkeytype repo, so not in scope of this source set. Needs a separate check (e.g. FSF guidance) and a user decision.
- Repo-quality notes (not relied on): `docs/THEMES.md` L93 links `./CONTRIBUTING.md#theme-guidelines`, which does not exist at repo root (real file is `docs/CONTRIBUTING.md`). Some headings in the clone render as "." (e.g. `docs/CONTRIBUTING.md` L1), so some doc text may be corrupted in this clone.
- Not run: no build, no tests, no repo scripts.

## Sources

Commit: `dd28fa71fb92638f29b0fc083895d953bfa65a7d` (2026-11-18T19:45:41+01:00)

- https://github.com/monkeytypegame/monkeytype/blob/dd28fa71fb92638f29b0fc083895d953bfa65a7d/LICENSE
- https://github.com/monkeytypegame/monkeytype/blob/dd28fa71fb92638f29b0fc083895d953bfa65a7d/package.json
- https://github.com/monkeytypegame/monkeytype/blob/dd28fa71fb92638f29b0fc083895d953bfa65a7d/frontend/package.json
- https://github.com/monkeytypegame/monkeytype/blob/dd28fa71fb92638f29b0fc083895d953bfa65a7d/README.md
- https://github.com/monkeytypegame/monkeytype/blob/dd28fa71fb92638f29b0fc083895d953bfa65a7d/docs/THEMES.md
- https://github.com/monkeytypegame/monkeytype/blob/dd28fa71fb92638f29b0fc083895d953bfa65a7d/docs/CONTRIBUTING.md
- https://github.com/monkeytypegame/monkeytype/blob/dd28fa71fb92638f29b0fc083895d953bfa65a7d/packages/schemas/src/themes.ts
- https://github.com/monkeytypegame/monkeytype/blob/dd28fa71fb92638f29b0fc083895d953bfa65a7d/packages/schemas/src/configs.ts
- https://github.com/monkeytypegame/monkeytype/blob/dd28fa71fb92638f29b0fc083895d953bfa65a7d/frontend/src/ts/constants/themes.ts
- https://github.com/monkeytypegame/monkeytype/blob/dd28fa71fb92638f29b0fc083895d953bfa65a7d/frontend/src/ts/utils/colors.ts
- https://github.com/monkeytypegame/monkeytype/blob/dd28fa71fb92638f29b0fc083895d953bfa65a7d/frontend/src/ts/controllers/theme-controller.ts
- https://github.com/monkeytypegame/monkeytype/blob/dd28fa71fb92638f29b0fc083895d953bfa65a7d/frontend/src/ts/components/core/Theme.tsx
- https://github.com/monkeytypegame/monkeytype/tree/dd28fa71fb92638f29b0fc083895d953bfa65a7d/frontend/static/themes
