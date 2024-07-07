#![forbid(unsafe_code)]
#![warn(clippy::pedantic)]
#![allow(clippy::default_trait_access)]
#![allow(clippy::wildcard_imports)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::enum_glob_use)]
#![cfg_attr(test, allow(dead_code, unused_imports))]

mod error;
mod extract;
mod openapi;
mod ratelimit;
mod route;
mod session;
mod trace;

use std::{net::SocketAddr, sync::Arc, time::Duration};

use aide::{axum::ApiRouter, openapi::OpenApi};
use argon2::Argon2;

use axum::http::header;
use axum::{
	body::Body, extract::Request, http::HeaderName, response::Response, Extension, Router,
	ServiceExt,
};

use tower::{Layer, ServiceBuilder};
use tower_governor::GovernorLayer;
use tower_http::normalize_path::NormalizePathLayer;
use tower_http::{
	cors::{self, CorsLayer},
	request_id::MakeRequestUuid,
	trace::TraceLayer,
	ServiceBuilderExt as _,
};
use tracing::{info, Span};

#[cfg(not(debug_assertions))]
pub use core::env;
#[cfg(debug_assertions)]
pub use dotenvy_macro::dotenv as env;

const X_REQUEST_ID: HeaderName = HeaderName::from_static("x-request-id");

pub type Database = sqlx::Pool<sqlx::Postgres>;

/// The shared application state.
///
/// This should contain all shared dependencies that handlers need to access,
/// such as a database connection pool, a hash configuratin (if it's consistent across the app),
/// or a cache layer.
///
/// For dependencies only used by a single handler, you can combine states instead.
#[derive(Clone, axum::extract::FromRef)]
pub struct AppState {
	pub database: Database,
	pub hasher: Argon2<'static>,
}

#[tokio::main]
async fn main() {
	let _guard = trace::init_tracing_subscriber();

	aide::gen::on_error(|error| {
		tracing::error!("{error}");
	});

	println!("{}", env!("DATABASE_URL"));

	let state = AppState {
		database: Database::connect(env!("DATABASE_URL"))
			.await
			.expect("failed to connect to database"),
		hasher: Argon2::default(),
	};

	let port = env!("PORT").parse().expect("PORT must be a number");
	let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
		.await
		.expect("failed to bind to port");

	info!("Listening on port {port}");

	let app = app(state);
	let app = NormalizePathLayer::trim_trailing_slash().layer(app);
	let service = ServiceExt::<Request>::into_make_service_with_connect_info::<SocketAddr>(app);

	axum::serve(listener, service).await.unwrap();
}

fn app(state: AppState) -> Router {
	let mut openapi = OpenApi::default();
	let (default, secure) = (ratelimit::default(), ratelimit::secure());

	ratelimit::cleanup_old_limits(&[&default, &secure]);

	let app = ApiRouter::new()
		.nest("/posts", route::post::routes())
		.nest("/keys", route::key::routes());

	#[cfg(not(test))]
	// All non-secure routes are rate-limited with a more relaxed configuration.
	let app = app.layer(GovernorLayer { config: default });

	let app = app
		.nest(
			"/auth",
			#[cfg(not(test))]
			route::auth::routes()
				// Authentication routes (and other sensitive routes) are rate-limited
				// with a more strict configuration.
				.layer(GovernorLayer { config: secure }),
			#[cfg(test)]
			route::auth::routes(),
		)
		.layer(
			CorsLayer::new()
				.allow_origin(cors::AllowOrigin::any())
				.allow_headers([header::AUTHORIZATION])
				.vary(Vec::new()),
		);

	let app = app.nest_service("/docs", openapi::routes());

	app.finish_api_with(&mut openapi, openapi::docs)
		.layer(
			ServiceBuilder::new()
				.layer(Extension(Arc::new(openapi)))
				.compression()
				.set_request_id(X_REQUEST_ID, MakeRequestUuid)
				.layer(
					TraceLayer::new_for_http()
						.make_span_with(|request: &Request<Body>| {
							let Some(request_id) = request.headers().get(X_REQUEST_ID) else {
								return tracing::error_span!("missing request_id");
							};

							tracing::info_span!(
								"request",
								request_id = ?request_id,
								method = %request.method(),
								uri = %request.uri(),
								version = ?request.version(),
							)
						})
						.on_response(
							|response: &Response<Body>, _latency: Duration, span: &Span| {
								let _guard = span.enter();
								let status = response.status();

								info!(
									status = %status,
									"response"
								);
							},
						),
				)
				.propagate_request_id(X_REQUEST_ID),
		)
		.with_state(state)
}

#[cfg(test)]
mod test {
	pub use super::Database;
	pub use serde_json::json;

	use axum::http::StatusCode;
	use axum_test::{TestServer, TestServerConfig};

	use super::*;

	/// Constructs a new router with the given database connection pool
	/// with rate-limiting and documentation disabled.
	///
	/// It contains various utilities for testing the application, see
	/// more at [`axum_test`].
	pub fn app(database: Database) -> TestServer {
		let config = TestServerConfig::builder().save_cookies().build();
		let state = AppState {
			database,
			hasher: Argon2::default(),
		};

		TestServer::new_with_config(super::app(state), config).unwrap()
	}

	#[sqlx::test]
	async fn test_index(pool: Database) {
		let app = app(pool);
		let response = app.get("/").await;

		assert_eq!(response.status_code(), StatusCode::NOT_FOUND);
		assert_eq!(response.text(), "");
	}
}
