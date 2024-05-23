use axum::extract::{Path, State};
use macros::route;
use uuid::Uuid;

use crate::{
	extract::{Json, Query, Session},
	openapi::tag,
	Database,
};

use super::{model, Error, RouteError};

/// Get own posts
/// Returns a paginated response of your posts, newest first.
#[route(tag = tag::POST)]
pub async fn get_user_posts(
	State(database): State<Database>,
	session: Session,
	Query(paginate): Query<model::PaginateInput>,
) -> Result<Json<Vec<model::Post>>, RouteError> {
	let posts = sqlx::query_as!(
		model::Post,
		r#"
			SELECT * FROM post
			WHERE user_id = $1
			ORDER BY created_at DESC
			LIMIT $2 OFFSET $3
		"#,
		session.user.id,
		paginate.limit(),
		paginate.offset(),
	)
	.fetch_all(&database)
	.await?;

	Ok(Json(posts))
}

/// Get all posts
/// Returns a paginated response of all posts, newest first.
#[route(tag = tag::POST)]
pub async fn get_posts(
	State(database): State<Database>,
	Query(paginate): Query<model::PaginateInput>,
) -> Result<Json<Vec<model::Post>>, RouteError> {
	let posts = sqlx::query_as!(
		model::Post,
		r#"
			SELECT * FROM post
			ORDER BY created_at DESC
			LIMIT $1 OFFSET $2
		"#,
		paginate.limit(),
		paginate.offset(),
	)
	.fetch_all(&database)
	.await?;

	Ok(Json(posts))
}

/// Get single post
/// Returns a single post by its unique id.
#[route(tag = tag::POST)]
pub async fn get_post(
	State(database): State<Database>,
	Path(post_id): Path<Uuid>,
) -> Result<Json<model::Post>, RouteError> {
	let post = sqlx::query_as!(
		model::Post,
		r#"
			SELECT * FROM post
			WHERE id = $1
		"#,
		post_id,
	)
	.fetch_optional(&database)
	.await?;

	Ok(Json(post.ok_or(Error::UnknownPost(post_id))?))
}

/// Create post
/// Creates a new post.
#[route(tag = tag::POST)]
pub async fn create_post(
	State(database): State<Database>,
	session: Session,
	Json(input): Json<model::CreatePostInput>,
) -> Result<Json<model::Post>, RouteError> {
	let post = sqlx::query_as!(
		model::Post,
		r#"
			INSERT INTO post (id, user_id, title, content)
			VALUES (DEFAULT, $1, $2, $3)
			RETURNING *
		"#,
		session.user.id,
		input.title,
		input.content,
	)
	.fetch_one(&database)
	.await?;

	Ok(Json(post))
}

/// Update post
/// Updates an existing post by its unique id.
#[route(tag = tag::POST)]
pub async fn update_post(
	State(database): State<Database>,
	session: Session,
	Path(post_id): Path<Uuid>,
	Json(input): Json<model::UpdatePostInput>,
) -> Result<Json<model::Post>, RouteError> {
	let post = sqlx::query_as!(
		model::Post,
		r#"
			UPDATE post
			SET title = COALESCE($1, title), content = COALESCE($2, content)
			WHERE id = $3 AND user_id = $4
			RETURNING *
		"#,
		input.title,
		input.content,
		post_id,
		session.user.id,
	)
	.fetch_optional(&database)
	.await?;

	Ok(Json(post.ok_or(Error::UnknownPost(post_id))?))
}

/// Delete post
/// Deletes an existing post by its unique id.
#[route(tag = tag::POST)]
pub async fn delete_post(
	State(database): State<Database>,
	session: Session,
	Path(post_id): Path<Uuid>,
) -> Result<(), RouteError> {
	let post = sqlx::query!(
		r#"
			DELETE FROM post
			WHERE id = $1 AND user_id = $2
		"#,
		post_id,
		session.user.id,
	)
	.execute(&database)
	.await?;

	if post.rows_affected() == 0 {
		return Err(Error::UnknownPost(post_id).into());
	}

	Ok(())
}
