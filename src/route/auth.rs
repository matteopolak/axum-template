use argon2::Argon2;
use axum::{
	body::Body,
	extract::State,
	http::{header, Response, StatusCode},
	response::IntoResponse,
	routing::{get, post},
};
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

use crate::{
	extract::{Json, Session},
	model, session, AppState, Database,
};

pub const KEY_LENGTH: usize = 32;

pub fn routes() -> axum::Router<AppState> {
	axum::Router::new()
		.route("/login", post(login))
		.route("/logout", get(logout))
		.route("/register", post(register))
		.route("/me", get(me))
}

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
	#[error("no session cookie")]
	NoSessionCookie,
	#[error("invalid session cookie")]
	InvalidSessionCookie,
	#[error("username already taken")]
	UsernameTaken,
	#[error("email already taken")]
	EmailTaken,
}

impl Error {
	pub fn status(&self) -> StatusCode {
		match self {
			Self::InvalidUsernameOrPassword
			| Self::NoSessionCookie
			| Self::InvalidSessionCookie => StatusCode::UNAUTHORIZED,
			Self::Argon(..) | Self::Cookie(..) => StatusCode::INTERNAL_SERVER_ERROR,
			Self::UsernameTaken | Self::EmailTaken => StatusCode::CONFLICT,
		}
	}
}

impl IntoResponse for Error {
	fn into_response(self) -> Response<Body> {
		crate::Error::from(self).into_response()
	}
}

#[derive(Deserialize, Validate)]
pub struct LoginInput {
	#[validate(email)]
	pub email: String,
	#[validate(length(min = 8, max = 128))]
	pub password: String,
}

#[derive(Deserialize, Validate)]
pub struct RegisterInput {
	#[validate(email)]
	pub email: String,
	#[validate(length(min = 8, max = 128))]
	pub password: String,
	#[validate(length(min = 3, max = 16))]
	pub username: String,
}

/// Hashes a password with Argon2, using the user's id as a salt.
/// Since this is only used for logging in and creating a new password,
/// the scope of this function can remain in here with no issues.
fn hash_password(
	hasher: &Argon2,
	password: &str,
	id: &Uuid,
) -> Result<[u8; KEY_LENGTH], argon2::Error> {
	let mut hash = [0; KEY_LENGTH];

	hasher.hash_password_into(password.as_bytes(), id.as_bytes(), &mut hash)?;
	Ok(hash)
}

/// Returns the authenticated user.
async fn me(session: Session) -> impl IntoResponse {
	Json(session.user)
}

/// Returns a session token, assuming the credentials are valid.
async fn login(
	State(state): State<AppState>,
	Json(auth): Json<LoginInput>,
) -> Result<impl IntoResponse, crate::Error> {
	let user = sqlx::query_as!(
		model::User,
		r#"SELECT * FROM "user" WHERE email = $1"#,
		auth.email
	)
	.fetch_one(&state.database)
	.await;

	let Ok(user) = user else {
		return Err(Error::InvalidUsernameOrPassword.into());
	};

	let hashed = hash_password(&state.hasher, &auth.password, &user.id).map_err(Error::Argon)?;

	if user.password != hashed {
		return Err(Error::InvalidUsernameOrPassword.into());
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

/// Logs out of the authenticated account.
async fn logout(
	State(database): State<Database>,
	session: Session,
) -> Result<impl IntoResponse, crate::Error> {
	sqlx::query!("DELETE FROM session WHERE id = $1", session.id)
		.execute(&database)
		.await?;

	// Clear the session cookie
	Ok([(header::SET_COOKIE, session::clear_cookie().to_string())])
}

/// Registers a new account, returning an associated session cookie.
async fn register(
	State(state): State<AppState>,
	Json(auth): Json<RegisterInput>,
) -> Result<impl IntoResponse, crate::Error> {
	let user_id = Uuid::new_v4();
	let hashed = hash_password(&state.hasher, &auth.password, &user_id).map_err(Error::Argon)?;

	let mut tx = state.database.begin().await?;

	sqlx::query_scalar!(
		r#"
      INSERT INTO "user" (id, email, username, password) VALUES ($1, $2, $3, $4) RETURNING id
    "#,
		user_id,
		auth.email,
		auth.username,
		&hashed
	)
	.fetch_one(&mut *tx)
	.await
	.map_err(|e| match e {
		sqlx::Error::Database(ref d) => match d.constraint() {
			Some("user_email_key") => Error::EmailTaken.into(),
			Some("user_username_key") => Error::UsernameTaken.into(),
			_ => crate::Error::Database(e),
		},
		e => crate::Error::Database(e),
	})?;

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
