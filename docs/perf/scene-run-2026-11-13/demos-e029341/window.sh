#!/bin/bash
# P0's quiet window: three 60 s runs of the racer, then the slow-frame
# and withheld-reading demos. Each line logs its start, end, exit code
# and /proc/loadavg.
B=~/.cache/w-p0b/bin
R=~/.cache/w-p0b/runs
cd ~/.cache/w-p0b/vapors || exit 1
load() { cut -d' ' -f1-3 /proc/loadavg; }
for name in run1:perf-real run2:perf-real run3:perf-real demo-slow:perf-slow-frame demo-withheld:perf-withheld; do
  label=${name%%:*}; bin=${name#*:}
  echo "$label start $(date +%T) load $(load)" >> $R/log.txt
  $B/$bin --scene-run > $R/$label.txt 2>&1
  code=$?
  echo "$label end $(date +%T) exit $code load $(load)" >> $R/log.txt
done
