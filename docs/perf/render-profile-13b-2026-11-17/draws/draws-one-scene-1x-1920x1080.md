draws-one-scene-1x-1920x1080.csv: 3733 frames (first 20 left out); mean GPU time per frame 3573.5 µs

| pass | event | per frame | mean µs per frame | share |
|---|---|---|---|---|
| page | draw vs 751aea2d fs 8b81f945 | 1.0 | 989.0 | 28% |
| scene | slow color clear | 1.0 | 755.4 | 21% |
| page | slow color clear | 1.0 | 754.2 | 21% |
| scene | draw indexed vs 66ecd23d fs ae52fe7e | 3.5 | 635.6 | 18% |
| page | draw vs 5fc23035 fs xc79091a | 4.0 | 239.0 | 7% |
| scene | hiz clear | 1.0 | 101.8 | 3% |
| outside passes | copy | 4.5 | 53.3 | 1% |
| page | draw vs 2da32033 fs 88034306 | 4.0 | 43.3 | 1% |
| outside passes | ccs partial resolve | 1.0 | 2.1 | 0% |
| scene | ccs color clear | 0.0 | 0.0 | 0% |
| page | ccs color clear | 0.0 | 0.0 | 0% |
