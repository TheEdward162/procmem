#!/bin/sh

case "$1" in
	deck)
		cargo build --package procmem_examples --target x86_64-unknown-linux-musl --release
	;;

	*)
		echo "usage: build.sh deck"
		exit 1
	;;
esac
