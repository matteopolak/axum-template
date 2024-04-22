# axum template

An easy-to-use, easy-to-expand template for [`axum`](https://github.com/tokio-rs/axum).

## Features

- Authentication + sessions + cookies
- JSON validation
- Clean and modular routing
- Logging with [`tracing`](https://github.com/tokio-rs/tracing)

## Getting started

Database models are located in [`src/model.rs`](/src/model.rs), custom extractors in [`src/extract.rs`](src/extract.rs), the main application error in [`src/error.rs`](src/error.rs), and various routes are placed in [`src/route`](src/route).

