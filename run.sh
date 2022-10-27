#!/bin/bash

MIN_DATE="1900-01-01"
MAX_DATE=$(date --rfc-3339='date')
OUTPUT_CSV="output.csv"
OUTPUT_TAR="fixtures/output.tar.lzma"
UWU_TARGET="yew-wu/index.html"
UWU_DIST="yew-wu/dist"
cargo run --bin cli -- query -o ${OUTPUT_CSV} ${MIN_DATE} ${MAX_DATE}
tar -c --lzma -f ${OUTPUT_TAR} ${OUTPUT_CSV}
trunk build --release ${UWU_TARGET}
UWU_WASM=($(ls -1 ${UWU_DIST} | grep -v index.html | tr '\n' ' '))
# <link rel="preload" href="/yew-wu-fcf1735969dc05e3_bg.wasm" as="fetch" type="application/wasm" crossorigin="">
# <link rel="modulepreload" href="/yew-wu-fcf1735969dc05e3.js"></head>
# <body><script type="module">import init from '/yew-wu-fcf1735969dc05e3.js';init('/yew-wu-fcf1735969dc05e3_bg.wasm');</script></body></html>
for str in ${UWU_WASM[@]}; do
  echo $str
done