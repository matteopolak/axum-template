use aide::axum::{
	routing::{get_with, post_with},
	ApiRouter,
};
use axum::http::StatusCode;

use crate::{error, AppState};

pub mod model;
pub mod route;

/// An error that can occur during authentication.
///
/// Note that the messages are presented to the client, so they should not contain
/// sensitive information.
#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("invalid username or password")]
	InvalidUsernameOrPassword,
	#[error("password validation error")]
	Argon(#[from] argon2::Error),
	#[error("cookie error: {0}")]
	Cookie(#[from] cookie::ParseError),
	#[error("no session cookie or api key")]
	NoSessionCookieOrApiKey,
	#[error("invalid session cookie")]
	InvalidSessionCookie,
	#[error("invalid api key")]
	InvalidApiKey,
	#[error("username already taken")]
	UsernameTaken,
	#[error("email already taken")]
	EmailTaken,
}

pub type RouteError = error::RouteError<Error>;

pub fn routes() -> ApiRouter<AppState> {
	use route::*;

	ApiRouter::new()
		.api_route("/login", post_with(login, login_docs))
		.api_route("/logout", get_with(logout, logout_docs))
		.api_route("/register", post_with(register, register_docs))
		.api_route(
			"/me",
			get_with(get_me, get_me_docs)
				.put_with(update_me, update_me_docs)
				.delete_with(delete_me, delete_me_docs),
		)
}

impl error::ErrorShape for Error {
	fn status(&self) -> StatusCode {
		match self {
			Self::InvalidUsernameOrPassword
			| Self::NoSessionCookieOrApiKey
			| Self::InvalidSessionCookie
			| Self::InvalidApiKey => StatusCode::UNAUTHORIZED,
			Self::Argon(..) | Self::Cookie(..) => StatusCode::INTERNAL_SERVER_ERROR,
			Self::UsernameTaken | Self::EmailTaken => StatusCode::CONFLICT,
		}
	}

	fn errors(&self) -> Vec<error::Message<'_>> {
		vec![error::Message {
			content: self.to_string().into(),
			field: None,
			details: None,
		}]
	}
}

#[cfg(test)]
mod test {
	use crate::test::*;

	#[sqlx::test]
	async fn test_signup_flow(pool: Database) {
		let app = app(pool);

		let response = app
			.post("/auth/register")
			.json(&json!({
				"email": "john@smith.com",
				"username": "john",
				"password": "hunter2hunter",
			}))
			.await;

		assert_eq!(response.status_code(), 200);

		assert!(response
			.header("set-cookie")
			.to_str()
			.unwrap()
			.contains("session="));

		let response = app
			.post("/auth/login")
			.json(&json!({
				"email": "john@smith.com",
				"password": "hunter2hunter",
			}))
			.await;

		assert_eq!(response.status_code(), 200);

		assert!(response
			.header("set-cookie")
			.to_str()
			.unwrap()
			.contains("session="));

		let response = app.get("/auth/me").await;

		assert_eq!(response.status_code(), 200);

		assert_eq!(response.json::<serde_json::Value>()["username"], "john");
	}
}
