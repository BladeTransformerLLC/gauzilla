#!/bin/sh

wasm-pack build --release --target web

for arg in "$@"
do
  if [ "$arg" = "sfz" ]; then
    sfz -r --coi
  fi
done
