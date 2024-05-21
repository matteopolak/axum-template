#![warn(clippy::pedantic)]
#![allow(clippy::default_trait_access)]

mod error;
mod extract;
mod model;
mod openapi;
mod ratelimit;
mod route;
mod session;
mod trace;

use std::{net::SocketAddr, sync::Arc, time::Duration};

use aide::{axum::ApiRouter, openapi::OpenApi};
use argon2::Argon2;

use axum::{
	body::Body, extract::Request, http::HeaderName, response::Response, Extension, ServiceExt,
};

use tower::{Layer, ServiceBuilder};
use tower_governor::GovernorLayer;
use tower_http::{
	cors::{self, CorsLayer},
	normalize_path::NormalizePathLayer,
	request_id::MakeRequestUuid,
	trace::TraceLayer,
	ServiceBuilderExt as _,
};
use tracing::Span;

const X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

pub type Database = sqlx::Pool<sqlx::Postgres>;
pub type AppState = State;

/// The shared application state.
///
/// This should contain all shared dependencies that handlers need to access,
/// such as a database connection pool, a hash configuratin (if it's expensive to create),
/// or a cache client.
///
/// For dependencies only used by a single handler, you can combine states instead.
#[derive(Clone, axum::extract::FromRef)]
pub struct State {
	pub database: Database,
	pub hasher: Argon2<'static>,
}

#[tokio::main]
async fn main() {
	let _guard = trace::init_tracing_subscriber();
	let _ = dotenvy::dotenv();

	aide::gen::on_error(|error| {
		tracing::error!("{}", error);
	});

	aide::gen::extract_schemas(true);

	let state = State {
		database: Database::connect(
			&std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
		)
		.await
		.expect("failed to connect to database"),
		hasher: Argon2::default(),
	};

	let mut openapi = OpenApi::default();
	let (default, secure) = (ratelimit::default(), ratelimit::secure());

	ratelimit::cleanup_old_limits(&[&default, &secure]);

	let app = ApiRouter::new()
		.nest("/posts", route::posts::routes())
		// All non-secure routes are rate-limited with a more relaxed configuration.
		.layer(GovernorLayer { config: default })
		.nest(
			"/auth",
			route::auth::routes()
				// Authentication routes (and other sensitive routes) are rate-limited
				// with a more strict configuration.
				.layer(GovernorLayer { config: secure }),
		)
		.layer(
			CorsLayer::new()
				.allow_origin(cors::AllowOrigin::any())
				.allow_headers([session::X_API_KEY])
				.vary(Vec::new()),
		)
		.nest_service("/docs", route::docs::routes())
		.finish_api_with(&mut openapi, openapi::docs)
		.layer(
			ServiceBuilder::new()
				.layer(Extension(Arc::new(openapi)))
				.compression()
				.set_request_id(X_REQUEST_ID, MakeRequestUuid)
				.layer(
					TraceLayer::new_for_http()
						.make_span_with(|request: &Request<Body>| {
							let request_id = request.headers().get(X_REQUEST_ID);

							tracing::debug_span!(
								"request",
								request_id = ?request_id,
								method = %request.method(),
								uri = %request.uri(),
								version = ?request.version(),
							)
						})
						.on_response(
							|response: &Response<Body>, latency: Duration, span: &Span| {
								let status = response.status();
								let request_id = response.headers().get(X_REQUEST_ID);

								span.record("status", status.as_u16());
								span.record("latency", latency.as_millis());

								tracing::debug!(
									status = %status,
									histogram.latency_ms = %latency.as_millis(),
									request_id = ?request_id,
									"response"
								);
							},
						),
				)
				.propagate_request_id(X_REQUEST_ID),
		)
		.with_state(state);

	let app = NormalizePathLayer::trim_trailing_slash().layer(app);

	let port = std::env::var("PORT").map_or_else(
		|_| 3000,
		|port| port.parse().expect("PORT must be a number"),
	);

	let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
		.await
		.expect("failed to bind to port");

	axum::serve(
		listener,
		ServiceExt::<Request>::into_make_service_with_connect_info::<SocketAddr>(app),
	)
	.await
	.unwrap();
}
