#!/bin/bash
valgrind --tool=callgrind --dump-instr=yes --simulate-cache=yes --collect-jumps=yes "$@" > out
callgrind=$(ls -altr | tail -n 1 | awk '{print $9 }')
qcachegrind $PWD/$callgrind
