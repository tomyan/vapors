# Test font

`DroidSansFallback-subset-6F22.ttf`: Droid Sans Fallback (Apache License
2.0, Google; `LICENSE-Apache-2.0.txt` and `NOTICE.txt` beside it) cut
down to one glyph, U+6F22 漢, so the per-character font fallback test
has a font with a CJK glyph on every machine without depending on
system fonts. It is 2 KB and used by tests only; nothing ships it.

Made on 2026-10-22 with fontTools 4.67.0 from Debian's
`fonts-droid-fallback` 1:6.0.1r16-1.1 (`DroidSansFallbackFull.ttf`):

```
python3 -m fontTools.subset DroidSansFallbackFull.ttf --unicodes=U+6F22 \
    --output-file=DroidSansFallback-subset-6F22.ttf --name-IDs='*' --no-hinting
```
