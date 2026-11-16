Revision 160b966372d6. Each figure is the median of 3 runs, one page a process; mount to frame 0 idle is each run's own sum (mount + App::start to idle + frame 0 to idle), then its median. Times on release; instructions on judged.

| rows | cache | mount to frame 0 idle µs | mount all µs | start all µs | start style µs | start layout µs | start paint µs | frame all µs | frame to idle µs |
|---|---|---|---|---|---|---|---|---|---|
| 1000 | cold | 207956.9 | 7097.3 | 196912.6 | 6659.1 | 57062.1 | 9546.4 | 2365.1 | 3969.9 |
| 1000 | warm | 97942.7 | 6956.4 | 83955.7 | 6511.4 | 57656.3 | 9365.1 | 2368.6 | 3976.4 |
| 3000 | cold | 376783.6 | 21307.9 | 351628.4 | 21805.7 | 172566.1 | 32416.7 | 2238.4 | 3847.3 |
| 3000 | warm | 267004.7 | 21143.5 | 242014.2 | 22024.9 | 175925.1 | 31968.9 | 2220.0 | 3832.7 |
| 10000 | cold | 1027200.5 | 70426.9 | 957063.5 | 81510.3 | 616867.8 | 124644.8 | 2409.4 | 3999.4 |
| 10000 | warm | 908503.6 | 69752.6 | 832759.0 | 80722.6 | 606026.1 | 126668.4 | 2359.4 | 3930.4 |

| rows | mount all | start style | start layout | start paint | start write | frame all |
|---|---|---|---|---|---|---|
| 1000 | 32776701 | 46607248 | 222252022 | 33193700 | 112631 | 7399081 |
| 3000 | 94722945 | 134959196 | 679645889 | 97426837 | 112161 | 7418678 |
| 10000 | 318842126 | 448313645 | 2291411341 | 326721669 | 111892 | 7421554 |
