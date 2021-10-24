#!/bin/bash
set -euxo pipefail

target/debug/crdt-genome --actor=0 --count=10 --base=8000 2>&1 | tee actor-0.log &
target/debug/crdt-genome --actor=1 --count=10 --base=8000 2>&1 > actor-1.log &
target/debug/crdt-genome --actor=2 --count=10 --base=8000 2>&1 > actor-2.log &
target/debug/crdt-genome --actor=3 --count=10 --base=8000 2>&1 > actor-3.log &
target/debug/crdt-genome --actor=4 --count=10 --base=8000 2>&1 > actor-4.log &
target/debug/crdt-genome --actor=5 --count=10 --base=8000 2>&1 > actor-5.log &
target/debug/crdt-genome --actor=6 --count=10 --base=8000 2>&1 > actor-6.log &
target/debug/crdt-genome --actor=7 --count=10 --base=8000 2>&1 > actor-7.log &
target/debug/crdt-genome --actor=8 --count=10 --base=8000 2>&1 > actor-8.log &
target/debug/crdt-genome --actor=9 --count=10 --base=8000 2>&1 > actor-9.log &
