use aide::axum::IntoApiResponse;
use argon2::Argon2;
use axum::{
	extract::State,
	http::{header, StatusCode},
	response::IntoResponse,
};
use macros::route;
use uuid::Uuid;

use crate::{
	extract::{Json, Session, SessionOrApiKey},
	openapi::tag,
	session, AppState, Database,
};

use super::{model, Error, RouteError};

pub const KEY_LENGTH: usize = 32;

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

/// Log in
/// Logs in to an account, returning an associated session cookie.
#[route(tag = tag::AUTH, response(status = 200, description = "Logged in successfully.", shape = "Json<model::Session>"))]
pub async fn login(
	State(state): State<AppState>,
	Json(auth): Json<model::LoginInput>,
) -> Result<impl IntoApiResponse, RouteError> {
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

	let session = sqlx::query_as!(
		model::Session,
		"INSERT INTO session (user_id) VALUES ($1) RETURNING *",
		user.id
	)
	.fetch_one(&state.database)
	.await?;

	let cookie = session::create_cookie(session.id);

	Ok(([(header::SET_COOKIE, cookie.to_string())], Json(session)))
}

/// Log out
/// Logs out of the authenticated account. If authenticated with an API key, it will be invalidated.
#[route(tag = tag::AUTH, response(status = 200, description = "Logged out successfully."), response(status = 204, description = "Authenticated with API key, no session to log out of."))]
pub async fn logout(
	State(database): State<Database>,
	session: Session,
) -> Result<impl IntoApiResponse, RouteError> {
	let SessionOrApiKey::Session(id) = session.id else {
		return Ok(StatusCode::NO_CONTENT.into_response());
	};

	sqlx::query!("DELETE FROM session WHERE id = $1", id)
		.execute(&database)
		.await?;

	// Clear the session cookie
	Ok((
		[(header::SET_COOKIE, session::clear_cookie().to_string())],
		StatusCode::NO_CONTENT,
	)
		.into_response())
}

/// Register account
/// Registers a new account, returning an associated session cookie.
#[route(tag = tag::AUTH, response(status = 200, description = "Registered successfully.", shape = "Json<model::Session>"))]
pub async fn register(
	State(state): State<AppState>,
	Json(auth): Json<model::RegisterInput>,
) -> Result<impl IntoApiResponse, RouteError> {
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
			_ => RouteError::from(e),
		},
		e => RouteError::from(e),
	})?;

	let session = sqlx::query_as!(
		model::Session,
		r#"
			INSERT INTO session (user_id) VALUES ($1) RETURNING *
		"#,
		user_id
	)
	.fetch_one(&mut *tx)
	.await?;

	tx.commit().await?;

	let cookie = session::create_cookie(session.id);

	Ok(([(header::SET_COOKIE, cookie.to_string())], Json(session)))
}

/// Get user
/// Returns the authenticated user.
#[route(tag = tag::AUTH)]
pub async fn get_me(session: Session) -> Json<model::User> {
	Json(session.user)
}

/// Update user
/// Updates the authenticated user.
#[route(tag = tag::AUTH)]
pub async fn update_me(
	State(state): State<AppState>,
	session: Session,
	Json(auth): Json<model::UpdateUserInput>,
) -> Result<Json<model::User>, RouteError> {
	let user = sqlx::query_as!(
		model::User,
		r#"
			UPDATE "user"
			SET email = COALESCE($1, email), username = COALESCE($2, username)
			WHERE id = $3
			RETURNING *
		"#,
		auth.email,
		auth.username,
		session.user.id
	)
	.fetch_one(&state.database)
	.await?;

	Ok(Json(user))
}

/// Delete user
/// Deletes the authenticated user and their related content. This action is irreversible.
#[route(tag = tag::AUTH)]
pub async fn delete_me(
	State(database): State<Database>,
	session: Session,
) -> Result<impl IntoApiResponse, RouteError> {
	sqlx::query!("DELETE FROM \"user\" WHERE id = $1", session.user.id)
		.execute(&database)
		.await?;

	// Clear the session cookie
	Ok((
		[(header::SET_COOKIE, session::clear_cookie().to_string())],
		StatusCode::NO_CONTENT,
	))
}
