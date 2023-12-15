#!/bin/sh

wasm-pack build --release --target web

sfz -r --coi
