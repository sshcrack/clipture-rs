#!/bin/bash

mkdir -p target/debug

cp obs-studio/build/rundir/RelWithDebInfo/lib/x86_64-linux-gnu/* target/debug/ -r
mkdir -p target/debug/obs-plugins/64bit
cp obs-studio/build/rundir/RelWithDebInfo/lib/x86_64-linux-gnu/obs-plugins/* target/debug/obs-plugins/64bit/ -r

mkdir -p target/debug/data
cp obs-studio/build/rundir/RelWithDebInfo/share/obs/* target/debug/data -r

cp obs-studio/build/rundir/RelWithDebInfo/bin/* target/debug/ -r