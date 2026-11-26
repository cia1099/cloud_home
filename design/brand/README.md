# Cloud Home brand directions (R-072 step 1)

Three candidate directions. Nothing is applied anywhere yet; `design/system.md` and the mockups are unchanged.

| | Direction | Idea | Colour added to system.md |
|---|---|---|---|
| A | `direction-a/` Hearth | house silhouette with a round window | none (existing blue) |
| B | `direction-b/` Roofbox | amber roof over a teal device with a status light | teal + amber (brand moments only) |
| C | `direction-c/` Open Ring | graphite ring, open, with a blue dot | none (neutrals + existing blue) |

Compare them in `index.html` (open the file directly in a browser; no server, no network). It shows light and dark, 16/32/64/180px, one-colour and app-icon variants, and a phone-width frame; resize the window to check the narrow layout.

## Per-direction files

`mark.svg`, `mark-dark.svg`, `mark-mono.svg`, `wordmark.svg`, `wordmark-dark.svg`, `app-icon.svg`, `app-icon-dark.svg`, `README.md` (rationale in English and 简体中文, tokens, clear space). The mono mark is #000; recolour with a CSS mask or `filter` for one-colour use on any background.

## Known limits

- The wordmark SVGs use live text in the system font stack (SF on Apple platforms, PingFang fallbacks), not outlined paths, because no font tooling was available in this session. Before the chosen direction ships, outline the text so it renders identically everywhere.
- App icons are unmasked 1024px squares with a flat gradient; no custom highlights or shadows, per `app-icons.md` (the system adds them). The tinted/clear appearance is derived by the system from the dark icon.
- PNG previews `previews/dir-{a,b,c}.png` (for the report site, each at most 300KB) are not rendered yet: export them from `index.html` once a renderer is available.
- Product name stays Latin ("Cloud Home") in all three languages; no Chinese wordmark is designed.
