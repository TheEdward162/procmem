#!/bin/sh

case "$1" in
	deck)
		# cargo build --package procmem_examples --target x86_64-unknown-linux-musl --release
		cargo build --package procmem_python --target x86_64-unknown-linux-gnu --release
		
		##config
		# [target.x86_64-unknown-linux-gnu]
		# linker = "zig-x86_64-linux-gnu"
		##linker
		# zig cc -target x86_64-linux-gnu "$@"

		# scp target/x86_64-unknown-linux-gnu/release/libprocmem_python.so deck@192.168.0.171:Documents/procmem/procmem.so
		# scp procmem_examples/python/repl.py deck@192.168.0.171:Documents/procmem/
	;;

	*)
		echo "usage: build.sh deck"
		exit 1
	;;
esac
