#!/bin/sh

rm -f plugin.zip
mkdir -p procmem-decky/bin

cp -r ./dist procmem-decky/
cp -r ../target/x86_64-unknown-linux-gnu/release/libprocmem_python.so procmem-decky/bin/procmem.so

7zz a plugin.zip package.json plugin.json main.py LICENSE procmem-decky/
rm -rf procmem-decky
