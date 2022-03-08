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

cp -r $1.toml $2.toml
IN_LINK=$(readlink $1.in)
if [ -n "${IN_LINK}" ]; then
    ln -s ${IN_LINK} $2.in
else
    cp -r $1.in $2.in
fi
cp -r $1.out $2.out
if [ -e "$1.stdout" ]; then
    cp -r $1.stdout $2.stdout
fi
if [ -e "$1.stderr" ]; then
    cp -r $1.stderr $2.stderr
fi
git add $2.*
