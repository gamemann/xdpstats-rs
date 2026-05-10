#!/bin/bash

TYPE=release
OUT=xdpstats

cp -f ./target/$TYPE/xdpstats /usr/bin/$OUT

echo "Installed xdpstats to /usr/bin/$OUT"