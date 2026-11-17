draws-one-scene-4x-own.csv: 3733 frames (first 20 left out); mean GPU time per frame 2997.1 µs

| pass | event | per frame | mean µs per frame | share |
|---|---|---|---|---|
| scene | blit | 1.0 | 552.3 | 18% |
| page | blit | 1.0 | 526.4 | 18% |
| page | draw vs 751aea2d fs 8b81f945 | 1.0 | 421.5 | 14% |
| scene | draw indexed vs 66ecd23d fs ae52fe7e | 3.5 | 371.7 | 12% |
| scene | slow color clear | 1.0 | 365.5 | 12% |
| page | slow color clear | 1.0 | 363.2 | 12% |
| page | draw vs 5fc23035 fs xc79091a | 4.0 | 242.2 | 8% |
| page | draw vs 2da32033 fs 88034306 | 4.0 | 55.3 | 2% |
| outside passes | copy | 4.5 | 52.8 | 2% |
| scene | hiz clear | 1.0 | 43.6 | 1% |
| outside passes | ccs partial resolve | 1.0 | 2.1 | 0% |
| scene | mcs color clear | 0.0 | 0.3 | 0% |
| page | mcs color clear | 0.0 | 0.2 | 0% |
