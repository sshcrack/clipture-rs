#!/bin/bash


export LIBOBS_PATH=$(readlink -f ./obs-studio/build/rundir/RelWithDebInfo/lib/x86_64-linux-gnu)
cargo run