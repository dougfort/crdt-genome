#!/bin/bash
set -euxo pipefail

target/debug/crdt-genome --actor=0 --count=3 --base=8000 2>&1 | tee actor-0.log &
target/debug/crdt-genome --actor=1 --count=3 --base=8000 2>&1 > actor-1.log &
target/debug/crdt-genome --actor=2 --count=3 --base=8000 2>&1 > actor-2.log &
