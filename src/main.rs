#![warn(clippy::pedantic)]

mod error;
mod extract;
mod model;
mod route;
mod session;

use argon2::Argon2;
use axum::Router;

pub use error::Error;

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
	dotenvy::dotenv().ok();

	let state = State {
		database: Database::connect(
			&std::env::var("DATABASE_URL").expect("DATABASE_URL must be set"),
		)
		.await
		.expect("failed to connect to database"),
		hasher: Argon2::default(),
	};

	let app = Router::new()
		.nest("/auth", route::auth::routes())
		.nest("/posts", route::posts::routes())
		.with_state(state);

	let port = std::env::var("PORT").map_or_else(
		|_| 3000,
		|port| port.parse().expect("PORT must be a number"),
	);

	let listener = tokio::net::TcpListener::bind(("127.0.0.1", port))
		.await
		.expect("failed to bind to port");

	tracing::info!("listening on port {}", port);

	axum::serve(listener, app).await.unwrap();
}
