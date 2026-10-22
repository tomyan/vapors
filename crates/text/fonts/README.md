# Bundled fonts

Geist and Geist Mono, the variable weight fonts from the official
release, embedded with `include_bytes!` so native and web render the
same files.

| File | Source file in the release | Axes |
|---|---|---|
| `Geist-wght.ttf` | `geist-font/Geist/variable/Geist[wght].ttf` | `wght` 100..900 |
| `GeistMono-wght.ttf` | `geist-font/GeistMono/variable/GeistMono[wght].ttf` | `wght` 100..900 |

Version: v1.8.2, published 2026-10-21.
Source: <https://github.com/vercel/geist-font/releases/download/v1.8.2/geist-font-v1.8.2.zip>
(sha256 `43ffb66912b1c49e40c371d96e86b3b3dab42db83eb2d0c8a17321210394c4c7`).

The files are renamed from `Name[wght].ttf` to `Name-wght.ttf` because
square brackets are glob characters in shells and in `include_bytes!`
paths they are easy to misread. The bytes are unchanged (sha256 of
`Geist-wght.ttf`: `98c7f53f48cb74b8b89e012e3902f0c11d12ae361c67db8da35d5969cfaa610d`,
`GeistMono-wght.ttf`: `4d2bad39b04d4e994500de6d2a70d2a74beb61de24cd127394294258f55e8c7f`).

The italic variable fonts are in the release and not bundled: nothing
in the UI uses italics yet and they would add 356 KB to the web
download. Neither family has an optical size axis.

Licence: SIL Open Font License 1.1, `OFL.txt` beside the fonts, copied
from the release unchanged.
