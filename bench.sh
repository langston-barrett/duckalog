#!/usr/bin/env bash

set -e

rm -f edge.csv
max="${2:-20000}"
for _ in $(seq 0 "${1:-10000}"); do
  printf "%s,%s\n" n$((RANDOM % max)) n$((RANDOM % max)) >> "edge.csv"
done

before=${SECONDS}
souffle examples/tc.dl
after=${SECONDS}
printf "souffle: %s\n" "$((after-before))s"


cargo build --quiet --release --example tc
before=${SECONDS}
./target/release/examples/tc  < edge.csv > path2.csv
after=${SECONDS}
printf "duckalog: %s\n" "$((after-before))s"

sort path.csv > path.sorted.csv
sort path2.csv > path2.sorted.csv
diff -u path.sorted.csv path2.sorted.csv
