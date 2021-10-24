# crdt-genome

## Synopsis

Experiments with [Rust CRDTs](https://github.com/rust-crdt/rust-crdt) using [Tokio](https://tokio.rs/) web application framework [Axum](https://github.com/tokio-rs/axum).

## Background

Exploring some ideas of [Martin Kleppmann](https://martin.kleppmann.com), particularly [Local-First software](https://martin.kleppmann.com/papers/local-first.pdf).

See this [Podcast](https://museapp.com/podcast/41-local-first-software/)

Also this [conference talk](https://www.youtube.com/watch?v=Exr0iY_D-vw&t=1s)

## Scenario

A group of [Axum](https://github.com/tokio-rs/axum) executeables represents a group of [Actors](https://docs.rs/crdts/7.0.0/crdts/trait.Actor.html).

Each actor process maintains a genome, represented by a [CRDT List](https://docs.rs/crdts/7.0.0/crdts/list/struct.List.html#).

```rust
pub struct Genome {
    genes: ListOfGenes,
}
```

Each process mutates its genome at random intervals and circulates a [CmRDT Op](https://docs.rs/crdts/7.0.0/crdts/trait.CmRDT.html#associatedtype.Op) using HTTP POST.

The goal is to observe every genome instance converging to a common value.

## Caveat

*This little system has serious shortcomings. If a process joins late, or drops
out and rejoins, or even loses a single POST request, it will never have the
full genome*.

## Execution

see `scripts/run-test.sh`

to run some actors, first bring the executeable up to date:

```bash
cargo build
```

Then run instances of the executeable

```bash
#!/bin/bash
set -euxo pipefail

target/debug/crdt-genome --actor=0 --count=3 --base=8000 2>&1 | tee actor-0.log &
target/debug/crdt-genome --actor=1 --count=3 --base=8000 2>&1 > actor-1.log &
target/debug/crdt-genome --actor=2 --count=3 --base=8000 2>&1 > actor-2.log &
```

```bash
USAGE:
    crdt-genome --actor <actor> --base <base> --count <count>

FLAGS:
    -h, --help       Prints help information
    -V, --version    Prints version information

OPTIONS:
    -a, --actor <actor>    the actor id of this server
    -b, --base <base>      base port number
    -c, --count <count>    The number of actors
```

## Verification

We want to verify that every genome is converging to the same value.

Since the system is constantly changing, there is no absolute way to do this.

We use HTTP GET to return a string representation of the genome.

```bash
$ curl localhost:8000/genome
40fd70816b4f664be2f28766
```

At a fixed interval, each process polls all the others and compares its
genome representation with theirs. For this test, we simply log the result.

```bash
$ grep "match" actor-1.log 
Oct 24 15:24:05.500 DEBUG crdt_genome: match count = 6
Oct 24 15:24:10.516 DEBUG crdt_genome: match count = 9
Oct 24 15:24:15.528 DEBUG crdt_genome: match count = 9
Oct 24 15:24:20.540 DEBUG crdt_genome: match count = 9
Oct 24 15:24:25.551 DEBUG crdt_genome: match count = 9
Oct 24 15:24:30.561 DEBUG crdt_genome: match count = 9
Oct 24 15:24:35.573 DEBUG crdt_genome: match count = 9
Oct 24 15:24:40.587 DEBUG crdt_genome: match count = 9
Oct 24 15:24:45.601 DEBUG crdt_genome: match count = 9
Oct 24 15:24:50.613 DEBUG crdt_genome: match count = 9
Oct 24 15:24:55.623 DEBUG crdt_genome: match count = 9
Oct 24 15:25:00.635 DEBUG crdt_genome: match count = 9
Oct 24 15:25:05.646 DEBUG crdt_genome: match count = 9
Oct 24 15:25:10.657 DEBUG crdt_genome: match count = 9
```
