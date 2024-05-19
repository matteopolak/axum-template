#![warn(clippy::pedantic)]
#![allow(clippy::default_trait_access)]

mod error;
mod extract;
mod model;
mod ratelimit;
mod session;

mod route {
	pub mod auth;
	pub mod docs;
	pub mod posts;
}

use std::{borrow::Cow, collections::HashMap, net::SocketAddr, sync::Arc};

use aide::{
	axum::ApiRouter,
	openapi::{self, OpenApi, SecurityScheme, Tag},
	transform::TransformOpenApi,
};
use argon2::Argon2;

use axum::{extract::Request, Extension, ServiceExt};
pub use error::Error;
use extract::Json;
use tower::{Layer, ServiceBuilder};
use tower_governor::GovernorLayer;
use tower_http::{
	compression::CompressionLayer,
	cors::{self, CorsLayer},
	normalize_path::NormalizePathLayer,
	trace::TraceLayer,
};

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
	tracing_subscriber::fmt::init();
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
		// All non-secure routes are rate-limited with a more relaxed configuration.
		.nest("/posts", route::posts::routes())
		.layer(GovernorLayer { config: default })
		// Authentication routes (and other sensitive routes) are rate-limited
		// with a more strict configuration.
		.nest(
			"/auth",
			route::auth::routes().layer(GovernorLayer { config: secure }),
		)
		.nest_service("/docs", route::docs::routes())
		.finish_api_with(&mut openapi, api_docs)
		.layer(
			ServiceBuilder::new()
				.layer(Extension(Arc::new(openapi)))
				.layer(CompressionLayer::new())
				.layer(CorsLayer::new().allow_origin(cors::AllowOrigin::any())),
		)
		.layer(TraceLayer::new_for_http())
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

fn api_docs(api: TransformOpenApi) -> TransformOpenApi {
	api.title("Axum Example Open API")
		.summary("An example Axum application")
		.description(include_str!("../README.md"))
		.tag(Tag {
			name: "auth".into(),
			description: Some("User authentication".into()),
			..Default::default()
		})
		.tag(Tag {
			name: "posts".into(),
			description: Some("Post management".into()),
			..Default::default()
		})
		.security_scheme(
			"Session",
			SecurityScheme::ApiKey {
				location: openapi::ApiKeyLocation::Cookie,
				name: session::COOKIE_NAME.into(),
				description: Some("A user authentication cookie".into()),
				extensions: Default::default(),
			},
		)
		.security_scheme(
			"API Key",
			SecurityScheme::ApiKey {
				location: openapi::ApiKeyLocation::Header,
				name: "X-API-Key".into(),
				description: Some("An API key".into()),
				extensions: Default::default(),
			},
		)
		.default_response_with::<Json<error::Message>, _>(|res| {
			res.example(error::Message {
				content: "error message".into(),
				field: Some("optional field".into()),
				details: Some(Cow::Owned({
					let mut map = HashMap::new();
					map.insert("key".into(), serde_json::json!("value"));
					map
				})),
			})
		})
}
