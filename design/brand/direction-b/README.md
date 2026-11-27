# Direction B: Roofbox

A warm roof line over a rounded box with one indicator light: a small device under a roof.

**Rationale (EN).** Cloud Home is a real object on a shelf, a drive with a status light, that belongs to a household. The mark pairs an amber roof stroke (the family) with a teal rounded body (the device) and a single cut-out light that says "on and connected". Two colours make it the most expressive and the easiest to tell apart from generic blue storage apps, and the teal-and-amber pair reads warm and domestic. The cost is a second brand colour next to the UI blue, and finer detail: the indicator light is under 1px at 16px, so the mark relies on the roof and the box shape there.

**说明（简体中文）。** Cloud Home 是放在架子上的一台真实设备：带指示灯的硬盘，属于一个家庭。标志用琥珀色的屋顶线条代表“家人”，青绿色的圆角机身代表“设备”，再镂空一盏指示灯，表示“已开机、已连接”。双色让它最有表现力，也最容易与常见的蓝色网盘区分，青绿加琥珀的组合温暖、有居家感。代价是在界面蓝之外多了一个品牌色，细节也更多：16px 时指示灯不足 1px，此时靠屋顶和机身的形状辨认。

## Tokens

| Token | Light | Dark | Note |
|---|---|---|---|
| `--brand` | #0d7f78 | #3cc5bb | 4.86:1 on white; about 8.0:1 on `--surface` dark |
| `--brand-warm` | #e08a00 | #ffb340 | graphics only (matches `--st-warn` dot); never text |
| `--brand-ink` | #1d1d1f | #f5f5f7 | wordmark, = `--text` |
| icon background | #17a89f to #0a6a64 (diagonal) | #16302e to #091514 | |
| icon foreground | body #ffffff, roof #ffc971 | body #3cc5bb, roof #ffb340 | |

UI keeps `--accent` blue for interaction; the teal is for brand moments only (icon, sign-in, empty states).

## Files

`mark.svg`, `mark-dark.svg`, `mark-mono.svg` (roof and body one colour, #000), `wordmark.svg`, `wordmark-dark.svg` ("Home" in brand colour), `app-icon.svg`, `app-icon-dark.svg`. Minimum mark size 16px; clear space = roof stroke width x 2.
