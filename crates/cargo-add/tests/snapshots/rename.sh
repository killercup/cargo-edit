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
if [ -e "$1.stdout" ]; then
    mv $1.stdout $2.stdout
fi
if [ -e "$1.stderr" ]; then
    mv $1.stderr $2.stderr
fi
git add $2.*
