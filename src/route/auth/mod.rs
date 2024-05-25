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
	#[error("invalid_username_or_password")]
	InvalidUsernameOrPassword,
	#[error("password_hash_error")]
	Argon(#[from] argon2::Error),
	#[error("cookie_parse_error")]
	Cookie(#[from] cookie::ParseError),
	#[error("authentication_required")]
	NoSessionCookieOrApiKey,
	#[error("invalid_session")]
	InvalidSessionCookie,
	#[error("invalid_api_key")]
	InvalidApiKey,
	#[error("username_taken")]
	UsernameTaken,
	#[error("email_taken")]
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

	fn into_errors(self) -> Vec<error::Message<'static>> {
		use Error::*;

		let message = match self {
			InvalidUsernameOrPassword => "An invalid username or password was provided.",
			Argon(..) => "An error occurred while hashing the password.",
			Cookie(..) => "An error occurred while parsing the cookie.",
			NoSessionCookieOrApiKey => "An authentication cookie or API key is required.",
			InvalidSessionCookie => "The provided session cookie is invalid.",
			InvalidApiKey => "The provided API key is invalid.",
			UsernameTaken => "The provided username is already taken.",
			EmailTaken => "The provided email is already taken.",
		};

		error::Message::new(self.to_string())
			.content(message)
			.into_vec()
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
