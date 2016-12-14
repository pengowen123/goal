#!/bin/bash

cargo build --release

sudo cp ./target/release/goal /usr/bin/
