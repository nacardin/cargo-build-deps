# cargo-build-deps

[![Build Status](https://travis-ci.org/nacardin/cargo-build-deps.svg?branch=master)](https://travis-ci.org/nacardin/cargo-build-deps)
[![Crates.io](https://img.shields.io/crates/v/cargo-build-deps.svg)](https://crates.io/crates/cargo-build-deps)

This tool extends [Cargo](https://doc.rust-lang.org/cargo/) to allow you to
build only the dependencies in a given rust project. This is useful for docker
builds where each build step is cached. The time it takes to build dependencies
is often a significant portion of the overall build time. Therefore it is
beneficial in docker builds to build dependencies in a separate step earlier
than the main build. Since the dependency building step will be cached,
dependencies will not need to be rebuilt when the project's own source code
changes.

Inspired by (http://atodorov.org/blog/2017/08/30/speeding-up-rust-builds-inside-docker/)


## Install
`cargo install cargo-build-deps`

## Usage
`cargo build-deps`

## Example

Change Dockerfile from

```
FROM rust:1 as rust-builder
RUN mkdir /tmp/PROJECT_NAME
WORKDIR /tmp/PROJECT_NAME
COPY . .
RUN cargo build  --release
```

to

```
FROM rust:1 as rust-builder
RUN cargo install cargo-build-deps
RUN cd /tmp && USER=root cargo new --bin PROJECT_NAME
WORKDIR /tmp/PROJECT_NAME
COPY Cargo.toml Cargo.lock ./
RUN cargo build-deps --release
COPY src /tmp/PROJECT_NAME/src
RUN cargo build  --release
```

## License

Licensed under either of these:

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or
   https://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or
   https://opensource.org/licenses/MIT)

### Contributing

Unless you explicitly state otherwise, any contribution you intentionally submit
for inclusion in the work, as defined in the Apache-2.0 license, shall be
dual-licensed as above, without any additional terms or conditions.