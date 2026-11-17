draws-one-scene-4x-1920x1080.csv: 3733 frames (first 20 left out); mean GPU time per frame 7053.6 µs

| pass | event | per frame | mean µs per frame | share |
|---|---|---|---|---|
| scene | blit | 1.0 | 1435.6 | 20% |
| page | blit | 1.0 | 1415.3 | 20% |
| page | draw vs 751aea2d fs 8b81f945 | 1.0 | 1074.8 | 15% |
| scene | slow color clear | 1.0 | 939.5 | 13% |
| page | slow color clear | 1.0 | 937.4 | 13% |
| scene | draw indexed vs 66ecd23d fs ae52fe7e | 3.5 | 793.4 | 11% |
| page | draw vs 5fc23035 fs xc79091a | 4.0 | 241.5 | 3% |
| scene | hiz clear | 1.0 | 102.1 | 1% |
| page | draw vs 2da32033 fs 88034306 | 4.0 | 55.6 | 1% |
| outside passes | copy | 4.5 | 55.0 | 1% |
| outside passes | ccs partial resolve | 1.0 | 2.0 | 0% |
| scene | mcs color clear | 0.0 | 0.7 | 0% |
| page | mcs color clear | 0.0 | 0.6 | 0% |
