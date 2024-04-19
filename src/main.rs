mod error;
mod extract;
mod model;
mod route;

use std::sync::Arc;

use argon2::Argon2;
use axum::routing::get;
use axum::Router;

pub type Database = sqlx::Pool<sqlx::Postgres>;
pub type AppState = Arc<State>;

#[derive(axum::extract::FromRef)]
pub struct State {
	pub database: Database,
	pub hasher: Argon2<'static>,
}

#[tokio::main]
async fn main() {
	dotenvy::dotenv().ok();

	let state = Arc::new(State {
		database: Database::connect("postgres://localhost/database")
			.await
			.unwrap(),
		hasher: Argon2::default(),
	});

	let app = Router::new()
		.nest("/auth", route::auth::routes(state.clone()))
		.with_state(state);

	let listener = tokio::net::TcpListener::bind(("127.0.0.1", 3000))
		.await
		.unwrap();

	axum::serve(listener, app).await.unwrap();
}

async fn index() -> &'static str {
	"Hello, World!"
}
