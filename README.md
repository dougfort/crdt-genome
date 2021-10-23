# crdt-genome

## Synopsis

Experiments with [Rust CRDTs](https://github.com/rust-crdt/rust-crdt) using [Axum](https://github.com/tokio-rs/axum).

Mutating a simple *genome* consisting of a list of numbers.

## Background

This project comes from my interest in the work of [Martin Kleppmann](https://martin.kleppmann.com/).

Particularly [Local-First software](https://martin.kleppmann.com/papers/local-first.pdf).

See this [Podcast](https://museapp.com/podcast/41-local-first-software/)

Also this [conference talk](https://www.youtube.com/watch?v=Exr0iY_D-vw&t=1s)

## Scenario

A group of Axum processes represents a group of Actors.

Each Actor maintains a genome, represented by a [CRDT List](https://docs.rs/crdts/7.0.0/crdts/list/struct.List.html#).

An Actor mutates its genome at random intervals and broadcasts a [CmRDT Op](https://docs.rs/crdts/7.0.0/crdts/trait.CmRDT.html#associatedtype.Op) to notify the other Actors of the change.

The goal is to observe every genome instance convergingto a common value.

## Configuration

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
