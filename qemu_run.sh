#!/bin/bash

# This requires you to previously run `cargo install defmt-print`

# See https://ferroussystems.hackmd.io/@jonathanpallant/ryA1S6QDJx for a description of all the relevant QEMU machines

ELF_BINARY=$1
shift
# All suitable for thumbv7em-none-eabi or thumbv7em-none-eabihf
MACHINE="-cpu cortex-m4 -machine mps2-an386"
LOG_FORMAT='{t} {[{L}]%bold} {s} {({ff}:{l:1})%dimmed}'
echo "ELF_BINARY=$ELF_BINARY"
echo "Running on '$MACHINE'..."
echo "------------------------------------------------------------------------"
echo qemu-system-arm $MACHINE -semihosting-config enable=on,target=native -nographic -kernel $ELF_BINARY $*
qemu-system-arm $MACHINE -semihosting-config enable=on,target=native -nographic -kernel $ELF_BINARY $* | defmt-print -e $ELF_BINARY --log-format="$LOG_FORMAT"
echo "------------------------------------------------------------------------"
