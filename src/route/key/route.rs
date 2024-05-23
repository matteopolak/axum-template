use axum::extract::{Path, State};
use macros::route;
use uuid::Uuid;

use crate::{
	extract::{Json, Query, Session},
	openapi::tag,
	AppState,
};

use super::{model, Error, RouteError};

/// List API keys
/// Lists all API keys associated with the authenticated user.
#[route(tag = tag::KEY)]
pub async fn list_keys(
	State(state): State<AppState>,
	session: Session,
	Query(paginate): Query<model::PaginateInput>,
) -> Result<Json<Vec<model::Key>>, RouteError> {
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
pub async fn create_key(
	State(state): State<AppState>,
	session: Session,
) -> Result<Json<model::Key>, RouteError> {
	let key = sqlx::query_as!(
		model::Key,
		r#"
			INSERT INTO api_keys (id, user_id) VALUES (DEFAULT, $1)
			RETURNING id, user_id, created_at
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
pub async fn get_key(
	State(state): State<AppState>,
	session: Session,
	Path(key_id): Path<Uuid>,
) -> Result<Json<model::Key>, RouteError> {
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
pub async fn delete_key(
	State(state): State<AppState>,
	session: Session,
	Path(key_id): Path<Uuid>,
) -> Result<(), RouteError> {
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
