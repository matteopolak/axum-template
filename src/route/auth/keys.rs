use std::borrow::Cow;

use aide::{
	axum::{routing::get_with, ApiRouter, IntoApiResponse},
	transform::TransformOperation,
};
use axum::{
	extract::{Path, State},
	http::StatusCode,
};
use serde_json::json;
use uuid::Uuid;

use crate::{
	error,
	extract::{Json, Session},
	model, AppState,
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("unknown key {0}")]
	UnknownKey(Uuid),
}

impl error::ErrorShape for Error {
	fn status(&self) -> StatusCode {
		match self {
			Self::UnknownKey(..) => StatusCode::NOT_FOUND,
		}
	}

	fn errors(&self) -> Vec<error::Message<'_>> {
		match self {
			Self::UnknownKey(key) => vec![error::Message {
				content: "unknown_key".into(),
				field: None,
				details: Some(Cow::Owned({
					let mut map = error::Map::new();
					map.insert("key".into(), json!(key));
					map
				})),
			}],
		}
	}
}

type RouteError = error::RouteError<Error>;

pub fn routes() -> ApiRouter<AppState> {
	ApiRouter::new()
		.api_route(
			"/keys",
			get_with(list_keys, list_keys_docs).post_with(create_key, create_key_docs),
		)
		.api_route(
			"/keys/:id",
			get_with(get_key, get_key_docs).delete_with(delete_key, delete_key_docs),
		)
}

fn list_keys_docs(op: TransformOperation) -> TransformOperation {
	op.summary("List all API keys")
		.description("Lists all API keys associated with the authenticated user.")
		.tag("auth")
}

/// Lists all API keys associated with the authenticated user.
async fn list_keys(
	State(state): State<AppState>,
	session: Session,
) -> Result<impl IntoApiResponse, RouteError> {
	let keys = sqlx::query_as!(
		model::Key,
		r#"
			SELECT * FROM api_keys WHERE user_id = $1
		"#,
		session.user.id
	)
	.fetch_all(&state.database)
	.await?;

	Ok(Json(keys))
}

fn create_key_docs(op: TransformOperation) -> TransformOperation {
	op.summary("Create a new API key")
		.description("Creates a new API key associated with the authenticated user.")
		.tag("auth")
}

/// Creates a new API key associated with the authenticated user.
async fn create_key(
	State(state): State<AppState>,
	session: Session,
) -> Result<impl IntoApiResponse, RouteError> {
	let key = sqlx::query_scalar!(
		r#"
			INSERT INTO api_keys (id, user_id) VALUES (DEFAULT, $1)
			RETURNING id
		"#,
		session.user.id
	)
	.fetch_one(&state.database)
	.await?;

	Ok(Json(key))
}

fn get_key_docs(op: TransformOperation) -> TransformOperation {
	op.summary("Get an API key")
		.description("Gets an API key associated with the authenticated user by id.")
		.tag("auth")
}

/// Gets an API key associated with the authenticated user by id.
async fn get_key(
	State(state): State<AppState>,
	session: Session,
	Path(key_id): Path<Uuid>,
) -> Result<impl IntoApiResponse, RouteError> {
	let key = sqlx::query_as!(
		model::Key,
		r#"
			SELECT * FROM api_keys WHERE id = $1 AND user_id = $2
		"#,
		key_id,
		session.user.id,
	)
	.fetch_optional(&state.database)
	.await?;

	Ok(Json(key.ok_or(Error::UnknownKey(key_id))?))
}

fn delete_key_docs(op: TransformOperation) -> TransformOperation {
	op.summary("Delete an API key")
		.description("Deletes an API key associated with the authenticated user.")
		.tag("auth")
}

/// Deletes an API key associated with the authenticated user.
async fn delete_key(
	State(state): State<AppState>,
	session: Session,
) -> Result<impl IntoApiResponse, RouteError> {
	let status = sqlx::query!(
		r#"
			DELETE FROM api_keys WHERE id = $1 AND user_id = $2
		"#,
		session.id,
		session.user.id
	)
	.execute(&state.database)
	.await?;

	if status.rows_affected() == 0 {
		return Err(Error::UnknownKey(session.id).into());
	}

	Ok(())
}
