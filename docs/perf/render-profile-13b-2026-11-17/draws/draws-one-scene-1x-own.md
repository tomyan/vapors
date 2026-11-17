draws-one-scene-1x-own.csv: 3733 frames (first 20 left out); mean GPU time per frame 1639.8 µs

| pass | event | per frame | mean µs per frame | share |
|---|---|---|---|---|
| page | draw vs 751aea2d fs 8b81f945 | 1.0 | 409.3 | 25% |
| scene | slow color clear | 1.0 | 294.8 | 18% |
| page | slow color clear | 1.0 | 294.2 | 18% |
| scene | draw indexed vs 66ecd23d fs ae52fe7e | 3.5 | 266.3 | 16% |
| page | draw vs 5fc23035 fs xc79091a | 4.0 | 235.8 | 14% |
| outside passes | copy | 4.5 | 52.1 | 3% |
| scene | hiz clear | 1.0 | 43.2 | 3% |
| page | draw vs 2da32033 fs 88034306 | 4.0 | 42.4 | 3% |
| outside passes | ccs partial resolve | 1.0 | 1.7 | 0% |
| scene | ccs color clear | 0.0 | 0.0 | 0% |
| page | ccs color clear | 0.0 | 0.0 | 0% |
