# Direction A: Hearth

A house silhouette with a round opening: the home device, drawn as one filled shape.

**Rationale (EN).** The product is a drive that lives in your house, so the mark is the house: a single soft-cornered silhouette with a round window cut through it, like a lit room. One filled shape survives 16px and any system effect, and it uses the product's existing blue, so brand and interface already agree. It is the most literal and friendliest option; its risk is familiarity, because house shapes are common, and it sits close to the "Home" glyph the app may use in navigation.

**说明（简体中文）。** 产品是放在家里的云盘，所以标志就是一座“家”：圆角的房屋剪影，中间镂空一个圆形窗口，像一间亮着灯的房间。单一实心形状在 16px 下依然清晰，也经得起系统的光影处理；颜色直接沿用界面现有的蓝色，品牌与界面天然一致。它最直白、最亲切；风险是房屋图形很常见，并且与应用内可能使用的“主页”图标相近。

## Tokens (consistent with `design/system.md`)

| Token | Light | Dark | Note |
|---|---|---|---|
| `--brand` | #0071e3 | #2997ff | same as `--accent-fill` / dark accent; 4.70:1 white on #0071e3 |
| `--brand-ink` | #1d1d1f | #f5f5f7 | wordmark text, = `--text` |
| icon background | #4aa3ff to #0066d6 (diagonal) | #2c2c2e to #0b0b0d | white on #0066d6 is about 5.4:1 |
| icon foreground | #ffffff | #64b5ff to #0a84ff | |

## Files

`mark.svg`, `mark-dark.svg`, `mark-mono.svg` (single colour, #000, recolour with CSS), `wordmark.svg`, `wordmark-dark.svg`, `app-icon.svg`, `app-icon-dark.svg` (1024px, unmasked square; the system applies the corner mask). Minimum mark size 16px; clear space around the mark = the opening diameter (about 0.3 of its height).
