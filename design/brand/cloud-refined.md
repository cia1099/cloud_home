# Cloud Home mark: cloud drive at home (R-072, round 2)

The owner denied variants A and B (plan/APPROVALS.md A-003) and asked: "Follow Apple SF Symbol style. I hope the brand can present cloud drive at home." This file replaces the earlier A/B notes. `cloud-a.svg`, `cloud-b.svg` and the A/B favicon/dark files are the denied directions and are still in the folder; they should be removed with `git rm` once the owner picks C or D (this session had no shell, so nothing was deleted).

The base is unchanged: the report-site cloud (same path, three lobes and a flat base, 1024 grid) on the brand-blue squircle tile. The two new variants add the "home" half of the idea using SF Symbols rules (`sf-symbols.md`, `icons.md`): one stroke weight per glyph, round caps and joins, simple geometry, optical balance, and a form that survives in a single colour.

## Variants

| | Idea | Style | Files |
|---|---|---|---|
| **C, Hearth** | A house sits inside the cloud: the drive is the cloud, home is what it holds. Reads like `cloud` + `house` composed as a symbol. | Outline. Cloud 46 u stroke, house 34 u (inner glyph lighter, as SF composed symbols do). Round caps and joins. | `cloud-c.svg`, `cloud-c-dark.svg`, `cloud-c-favicon.svg` |
| **D, Roofline** | The cloud is the house: a roof chevron over a solid cloud body, with an arched doorway cut through the cloud's base. A cloud wearing a roof is a home. | Filled body plus one 56 u stroke for the roof, same round terminals. The door is a true knock-out (SVG mask), so it works on any background. | `cloud-d.svg`, `cloud-d-dark.svg`, `cloud-d-favicon.svg` |

Units are viewBox units on the 1024 tile. Favicons are heavier and simplified: C becomes a solid white cloud with a solid blue house cut into it (no outline, no gradient, no filter); D uses an 84 u roof stroke and a wider 72 u door. Both are flat #1668e0 with white, about 5.4:1, above the 3:1 graphic-object floor.

## Light, dark, monochrome

- Light: blue tile #4aa8ff to #0a58d0, white glyph (D body gets a #fff to #dcebff gradient), one soft shadow under the glyph.
- Dark: graphite tile #3a3a3c to #1c1c1e, glyph in blue #6cb8ff to #2a7af0 (same pairing the old B-dark used). Blue on graphite is above 4.5:1 at its darkest stop.
- Monochrome: both glyphs are one colour with no inner fill tricks (D's door is a cut, C's house is a stroke), so they work as a template glyph in the product's `currentColor` icon slot or a one-colour print. Strip the tile and gradients, keep the glyph paths.

## Recommendation

D at small sizes (favicon, tab, 16 to 24 px): the roof and door read as one bold silhouette, and "home" is visible even at 16 px. C at large sizes (logo, sign-in, OG image): it is the more explicit "cloud with a house in it" and the house detail needs about 48 px to resolve. If one mark must serve every size, choose D. Unrendered caveat below applies to both.

## Rules (carried over)

- No text in the mark; the wordmark "Cloud Home" is set in the system font.
- OS app icon: supply a full-bleed square and let the system mask it (`app-icons.md`); replace the tile path with a plain rect. Clear space: about 12% of the tile.
- Minimum 16 px using the favicon file. Do not add a second brand colour.
- When several SVGs are inlined on one page, `id`s must be unique. The ids here are prefixed per file (`c-`, `cd-`, `d-`, `dd-`, `df-`).

## Not yet done

- Not rendered: there was no shell in this session, so there are no PNG previews. Check each SVG at 16, 32 and 256 px in a browser, `rsvg-convert` or `qlmanage -t -s 256 file.svg`. Likely tuning points: the house/roof proportions in C, and the door width in D at 16 px.
- Corner curve is the same cubic approximation of a continuous corner used before; review before shipping an OS icon.
- Applying the chosen mark to `manager-site/` (LOGO constant, favicon) is for the frontend. `design/brand/README.md` still lists the old A-001 directions and should be marked superseded.
