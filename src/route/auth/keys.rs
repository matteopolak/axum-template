use std::borrow::Cow;

use aide::axum::{routing::get_with, ApiRouter, IntoApiResponse};
use axum::{
	extract::{Path, State},
	http::StatusCode,
};
use macros::route;
use serde_json::json;
use uuid::Uuid;

use crate::{
	error,
	extract::{Json, Query, Session},
	model,
	openapi::tag,
	route, AppState,
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

/// List API keys
/// Lists all API keys associated with the authenticated user.
#[route(tag = tag::KEY)]
async fn list_keys(
	State(state): State<AppState>,
	session: Session,
	Query(paginate): Query<route::PaginateInput>,
) -> Result<impl IntoApiResponse, RouteError> {
	let keys = sqlx::query_as!(
		model::Key,
		r#"
			SELECT * FROM api_keys WHERE user_id = $1
			ORDER BY created_at DESC
			LIMIT $2 OFFSET $3
		"#,
		session.user.id,
		paginate.limit(),
		paginate.offset()
	)
	.fetch_all(&state.database)
	.await?;

	Ok(Json(keys))
}

/// Create API key
/// Creates a new API key associated with the authenticated user.
#[route(tag = tag::KEY)]
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

/// Get API key
/// Gets an API key associated with the authenticated user by id.
#[route(tag = tag::KEY)]
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

/// Delete API key
/// Deletes an API key associated with the authenticated user by id.
#[route(tag = tag::KEY)]
async fn delete_key(
	State(state): State<AppState>,
	session: Session,
	Path(key_id): Path<Uuid>,
) -> Result<impl IntoApiResponse, RouteError> {
	let status = sqlx::query!(
		r#"
			DELETE FROM api_keys WHERE id = $1 AND user_id = $2
		"#,
		key_id,
		session.user.id
	)
	.execute(&state.database)
	.await?;

	if status.rows_affected() == 0 {
		return Err(Error::UnknownKey(key_id).into());
	}

	Ok(())
}
