#!/bin/sh

flavor=${1:-debug}

setcap CAP_SYS_PTRACE+pe "target/${flavor}/string_finder"