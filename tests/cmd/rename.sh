#!/usr/bin/env bash
BEFORE=$1
if [ -z "${BEFORE}" ]; then
    echo "Usage: $0 <BEFORE> <AFTER>"
    exit 1
fi
AFTER=$2
if [ -z "${AFTER}" ]; then
    echo "Usage: $0 <BEFORE> <AFTER>"
    exit 1
fi

mv $1.in $2.in
mv $1.out $2.out
mv $1.stdout $2.stdout
mv $1.stderr $2.stderr
mv $1.toml $2.toml
git add $2.*
