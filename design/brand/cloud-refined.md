# Cloud Home mark: the report-site cloud, refined (R-072 redo)

Supersedes the A/B/C directions (denied by the owner: "I can't feel Apple design, not elegant. I like that icon: it represents a cloud."). The base is `LOGO` in `manager-site/src/shared/icons.ts`: blue tile, white cloud. The cloud path is kept as drawn (same three lobes, same flat base, scaled x36.57 to a 1024 grid); only the finish changes.

## What changed, and why

| Was | Now | Why |
|---|---|---|
| Rounded rectangle, rx 7/28 (circular corners) | Squircle: continuous-curvature corner (cubic path, control points at 0.72 of the half-width) | Apple icons use continuous corners; circular arcs show a visible tangent break and read as "web". |
| Flat white cloud | Vertical gradient #fff to #dcebff, plus a soft blue lower-edge inner shade | Gives the cloud a quiet volume without a drawn outline. |
| No depth | One soft drop shadow (dy 22, blur 26, #04307a at 38%) | Lifts the cloud off the tile; single light source from above. |
| Gradient #42a5ff to #0a5fd8 | #4aa8ff to #0a58d0, plus a faint radial sheen at the top-left | Same blue, slightly deeper end so the white cloud keeps contrast. |

## Variants

- **A, Sky** (`cloud-a.svg`): the owner's icon, finished. Blue tile, white cloud. Use for the report-site header logo, social/OG images and the product app icon.
- **A small** (`cloud-a-favicon.svg`): same shape with no filter, sheen or inner shade, and the cloud enlarged 1.18x so it fills more of the tile. Use for the favicon at 16/32px. White on the mid-tile blue is about 4.5:1 or better and 6.4:1 on the darkest stop; the graphic-object floor is 3:1.
- **B, Porcelain** (`cloud-b.svg`, `cloud-b-dark.svg`): inverse treatment, a pale (dark: graphite) tile with a blue gradient cloud and a top highlight. Quieter and closer to a system-app feel; use where the blue tile would be too loud, such as the sign-in screen or a monochrome-friendly context. The dark file swaps the tile to #3a3a3c to #1c1c1e and lightens the cloud to #6cb8ff to #2a7af0.

Recommendation: A for the logo and favicon (it is the icon the owner said they like, so keep the white-on-blue identity), B only as an alternate.

## Rules

- The tiles here are 1024 squircles for the site logo. For an OS app icon, supply a full-bleed square and let the system mask it (`app-icons.md`): the same artwork with the tile path replaced by a plain rect.
- No text in the mark. The wordmark stays "Cloud Home" in the system font.
- Clear space: one cloud-lobe radius (about 12% of the tile) on all sides. Minimum size 16px, and below 24px use the small file.
- One colour means one thing: this blue is the brand blue and already the interactive accent in `design/system.md`; do not add a second brand colour.
- Dark mode: A works unchanged on dark and light backgrounds; B has a dark file.

## Not yet done

- No PNG renders (no shell in this session); the manager will render and commit. Check the 16px result of `cloud-a-favicon.svg` visually.
- Corner curve is a cubic approximation of a continuous corner, not Apple's exact curve; adequate for web, review before shipping an OS icon.
- `design/brand/README.md` (A-001 notes) still lists directions A/B/C. It should be marked superseded by this file and the `direction-*` folders removed once the owner approves the refined cloud, per the no-leftovers rule.
- Applying it to `manager-site` (LOGO constant, favicon) is for the frontend; the replacement is the contents of `cloud-a.svg` / `cloud-a-favicon.svg`. Note `id`s (`tile`, `cloud`, ...) must be unique if inlined more than once on a page.
