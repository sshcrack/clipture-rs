#!/bin/bash

cp obs-studio/build/rundir/RelWithDebInfo/lib/x86_64-linux-gnu/ target/debug/ -r
mkdir target/debug/share
cp obs-studio/build/rundir/RelWithDebInfo/share target/debug/share -r

cp obs-studio/build/rundir/RelWithDebInfo/bin/* target/debug/ -r