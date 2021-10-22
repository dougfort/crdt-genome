# crdt-genome

## Synopsis

Experiments with [Rust CRDTs](https://github.com/rust-crdt/rust-crdt) using [Axum](https://github.com/tokio-rs/axum).

Mutating a simple *genome* consisting of a list of numbers.

## Background

This project comes from my interest in the work of [Martin Kleppmann](https://martin.kleppmann.com/).

Particularly [Local-First software](https://martin.kleppmann.com/papers/local-first.pdf).

See this [Podcast](https://museapp.com/podcast/41-local-first-software/)

Also this [conference talk](https://www.youtube.com/watch?v=Exr0iY_D-vw&t=1s)

## Configuration

This is a simple demo system. The scenario is N Actors all modifying a CRDT List
of u8 items.

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
