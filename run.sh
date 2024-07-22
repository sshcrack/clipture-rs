#!/bin/bash


export LIBOBS_PATH=$(readlink -f ./target/debug)
cargo run