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

## Libraries

- [`axum`](https://github.com/tokio-rs/axum) - Web framework
- [`sqlx`](https://github.com/launchbadge/sqlx) - SQL queries and migrations
- [`tracing`](https://github.com/tokio-rs/tracing) - Structured logging
- [`opentelemetry`](https://github.com/open-telemetry/opentelemetry-rust) et al. - Distributed tracing
- [`validator`](https://github.com/Keats/validator) - Input validation
- [`schemars`](https://github.com/GREsau/schemars) - OpenAPI schema generation
- [`aide`](https://github.com/tamasfe/aide) - API documentation
- [`governor`](https://github.com/boinkor-net/governor) - Rate limiting
- [`tower_http`](https://github.com/tower-rs/tower-http) - Various middleware

## Macros

This library includes a few macros to make your life easier. Here's the usage and output of each:

```rust
/// My first route
/// Fetches some data and gives it to you.
#[route(
  tag = "mytag",
  response(status = 200, description = "Something happened", shape = "Json<MyType>")
)] // <- This is the macro
async fn my_route() -> Result<Json<MyType>, Error> {
  Ok(Json(MyType))
}

// Creates an additional function:

use aide::transform::TransformOperation;

fn my_route_docs(op: TransformOperation) -> TransformOperation {
  op.summary("My first route")
    .description("Fetches some data and gives it to you.")
    .response_with::<200, Json<MyType>(|res| res.description("Something happened"))
    .tag("mytag")
}
```

```rust
#[model] // <- This is the macro
#[derive(Serialize, Deserialize, JsonSchema, Validate)] // etc.
struct Post {
  #[serde(skip_deserializing)] // <- Important! This is skipped.
  id: Uuid,
  #[validate(length(min = 1, max = 100))]
  title: String,
  content: String,
}

// Creates two additional structs:

#[derive(Serialize, Deserialize, JsonSchema, Validate)]
struct CreatePostInput {
  #[validate(length(min = 1, max = 100))]
  title: String,
  content: String,
}

#[derive(Serialize, Deserialize, JsonSchema, Validate)]
struct UpdatePostInput {
  #[validate(length(min = 1, max = 100))]
  title: Option<String>,
  content: Option<String>,
}
```

