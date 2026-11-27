# Direction C: Open Ring

A graphite ring, open on the right, with one blue dot in the opening.

**Rationale (EN).** The ring is a "C" for Cloud and a circle of people around a table; the dot in its opening is your home device, always on, always reachable. It is the quietest of the three and the closest to Apple's own restraint: neutral graphite, one small spot of the existing UI blue, no house or cloud cliche. Because it is not a literal house it will not collide with a "Home" icon in navigation, it is crisp at 16px (a 2.25px ring), and it dims gracefully into the mono and tinted icon appearances. The trade-off is that the meaning is learned, not obvious; the wordmark carries the name.

**说明（简体中文）。** 圆环既是 Cloud 的 “C”，也像一家人围坐的圆桌；开口处的圆点就是你家里的设备，始终在线、随时可达。它是三个方案里最安静的，也最贴近 Apple 式的克制：中性的石墨色加一点界面现有的蓝色，没有房子，也没有云的老套图形。因为不是字面意义上的房子，不会与导航里的“主页”图标冲突；16px 下环宽约 2.25px，依然清晰；在单色与着色图标外观下也很从容。代价是含义需要学习而非一眼可见，由文字标识负责传达名称。

## Tokens

| Token | Light | Dark | Note |
|---|---|---|---|
| `--brand-ink` | #1d1d1f | #f5f5f7 | ring and wordmark, = `--text`; 16.3:1 on `--bg` |
| `--brand-dot` | #0071e3 | #2997ff | = `--accent-fill`; non-text 4.7:1 on white |
| icon background | #ffffff to #e3e3e8 | #2c2c2e to #0b0b0d | |
| icon foreground | ring #1d1d1f, dot #0071e3 | ring #f5f5f7, dot #2997ff | |

No new colour is introduced: the brand is the system's own neutrals plus the existing accent.

## Files

`mark.svg`, `mark-dark.svg`, `mark-mono.svg` (single colour, #000), `wordmark.svg`, `wordmark-dark.svg` ("Cloud" 700, "Home" 300), `app-icon.svg`, `app-icon-dark.svg`. Minimum mark size 16px; clear space = ring stroke width x 1.5.
