use argon2::Argon2;
use axum::{
	body::Body,
	extract::State,
	http::{header, Response},
	response::IntoResponse,
	routing::{get, post},
};
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

use crate::{
	error::Error,
	extract::{Json, Session},
	model, session, AppState, Database,
};

pub const KEY_LENGTH: usize = 32;

pub fn routes() -> axum::Router<AppState> {
	axum::Router::new()
		.route("/login", post(login))
		.route("/logout", get(logout))
		.route("/register", post(register))
}

/// An error that can occur during authentication.
///
/// Note that the messages are presented to the client, so they should not contain
/// sensitive information.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
	#[error("invalid username or password")]
	InvalidUsernameOrPassword,
	#[error("password validation error")]
	Argon(#[from] argon2::Error),
	#[error("cookie error: {0}")]
	Cookie(#[from] cookie::ParseError),
	#[error("no session cookie")]
	NoSessionCookie,
	#[error("invalid session cookie")]
	InvalidSessionCookie,
}

impl IntoResponse for AuthError {
	fn into_response(self) -> Response<Body> {
		Error::from(self).into_response()
	}
}

#[derive(Deserialize, Validate)]
pub struct Auth {
	#[validate(email)]
	pub email: String,
	#[validate(length(min = 8, max = 128))]
	pub password: String,
}

fn hash_password(
	hasher: &Argon2,
	password: &str,
	id: &Uuid,
) -> Result<[u8; KEY_LENGTH], argon2::Error> {
	let mut hash = [0; KEY_LENGTH];

	hasher.hash_password_into(password.as_bytes(), id.as_bytes(), &mut hash)?;
	Ok(hash)
}

async fn login(
	State(state): State<AppState>,
	Json(auth): Json<Auth>,
) -> Result<impl IntoResponse, Error> {
	let user = sqlx::query_as!(
		model::User,
		r#"SELECT * FROM "user" WHERE email = $1"#,
		auth.email
	)
	.fetch_one(&state.database)
	.await;

	let Ok(user) = user else {
		return Err(AuthError::InvalidUsernameOrPassword.into());
	};

	let hashed =
		hash_password(&state.hasher, &auth.password, &user.id).map_err(AuthError::Argon)?;

	if user.password != hashed {
		return Err(AuthError::InvalidUsernameOrPassword.into());
	}

	let session_id = sqlx::query_scalar!(
		"INSERT INTO session (user_id) VALUES ($1) RETURNING id",
		user.id
	)
	.fetch_one(&state.database)
	.await?;

	let cookie = session::create_cookie(session_id);

	Ok([(header::SET_COOKIE, cookie.to_string())])
}

async fn logout(
	State(database): State<Database>,
	session: Session,
) -> Result<impl IntoResponse, Error> {
	sqlx::query!("DELETE FROM session WHERE id = $1", session.id)
		.execute(&database)
		.await?;

	// Clear the session cookie
	Ok([(header::SET_COOKIE, session::clear_cookie().to_string())])
}

async fn register(
	State(state): State<AppState>,
	Json(auth): Json<Auth>,
) -> Result<impl IntoResponse, Error> {
	let user_id = Uuid::new_v4();
	let hashed =
		hash_password(&state.hasher, &auth.password, &user_id).map_err(AuthError::Argon)?;

	let mut tx = state.database.begin().await?;

	sqlx::query_scalar!(
		r#"
      INSERT INTO "user" (id, email, password) VALUES ($1, $2, $3) RETURNING id
    "#,
		user_id,
		auth.email,
		&hashed
	)
	.fetch_one(&mut *tx)
	.await?;

	let session_id = sqlx::query_scalar!(
		r#"
      INSERT INTO session (user_id) VALUES ($1) RETURNING id
    "#,
		user_id
	)
	.fetch_one(&mut *tx)
	.await?;

	tx.commit().await?;

	let cookie = session::create_cookie(session_id);

	Ok([(header::SET_COOKIE, cookie.to_string())])
}
