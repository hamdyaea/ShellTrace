#!/bin/sh
set -eu
mkdir -p /tmp/shelltrace-demo
printf 'Before\n' > /tmp/shelltrace-demo/existing.txt
cargo run -- run --scope /tmp/shelltrace-demo --json /tmp/shelltrace-demo.json -- \
  /bin/sh -c 'printf "After\n" > /tmp/shelltrace-demo/existing.txt; touch /tmp/shelltrace-demo/new.txt'
