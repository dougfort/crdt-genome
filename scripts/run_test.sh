set -euxo pipefail

target/debug/crdt-genome --actor=0 --count=2 --base=8000 2>&1 | tee actor-0.log &
target/debug/crdt-genome --actor=1 --count=2 --base=8000 2>&1 > actor-1.log &
