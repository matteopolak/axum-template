# axum template

An easy-to-use, easy-to-expand template for [`axum`](https://github.com/tokio-rs/axum).

## Features

- Authentication + sessions with cookies
- Input validation for request body and query parameters
- Clean and modular routing
- Logging and tracing with OpenTelemetry
- Scalable error handling
- Ratelimiting, compression, and other middleware
- OpenAPI schema generation and documentation with Scalar

## Getting started

Database models are located in [`src/model.rs`](/src/model.rs), custom extractors in [`src/extract.rs`](src/extract.rs), the main application error in [`src/error.rs`](src/error.rs), and various routes are placed in [`src/route`](src/route).

