# Sourced by the timed-run scripts (scene-runs.sh, first-frame-runs.sh),
# with $out set to the run directory.
# load: the three load averages.
# settle: waits for a 1-minute load under 1.5, at most 900 s (the wait
# measured 400 s and 521 s after gates, 2026-11-14), and logs the wait,
# the load and the five busiest processes to $out/log.txt.
load() { cut -d' ' -f1-3 /proc/loadavg; }
settle() {
  started=$(date +%s)
  until awk "BEGIN { exit !($(cut -d' ' -f1 /proc/loadavg) < 1.5) }"; do
    if [ $(( $(date +%s) - started )) -ge 900 ]; then
      echo "the 1-minute load is still $(load) after 900 s; nothing was run" | tee -a "$out/log.txt" >&2; exit 1
    fi
    sleep 10
  done
  echo "settled after $(( $(date +%s) - started )) s, load $(load); busiest:" >> "$out/log.txt"
  ps -eo pcpu,etime,comm --sort=-pcpu | head -6 >> "$out/log.txt"
}
