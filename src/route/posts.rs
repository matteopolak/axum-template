use axum::{
	body::Body,
	extract::{Path, State},
	http::Response,
	response::IntoResponse,
	routing::{get, post},
};
use serde::Deserialize;
use uuid::Uuid;
use validator::Validate;

use crate::{
	error::Error,
	extract::{Json, Query, Session},
	model, AppState, Database,
};

pub fn routes() -> axum::Router<AppState> {
	axum::Router::new()
		.route("/", get(get_all_posts))
		.route("/me", post(get_user_posts))
		.route("/:id", post(get_one_post))
}

#[derive(Debug, thiserror::Error)]
pub enum PostError {
	#[error("unknown post {0}")]
	UnknownPost(Uuid),
}

impl IntoResponse for PostError {
	fn into_response(self) -> Response<Body> {
		Error::from(self).into_response()
	}
}

fn one() -> i64 {
	1
}

fn ten() -> i64 {
	10
}

#[derive(Deserialize, Validate)]
pub struct PostPaginateInput {
	#[validate(range(min = 1, max = 100))]
	#[serde(default = "one")]
	pub page: i64,
	#[validate(range(min = 1, max = 100))]
	#[serde(default = "ten")]
	pub size: i64,
}

pub async fn get_user_posts(
	State(database): State<Database>,
	session: Session,
	Query(paginate): Query<PostPaginateInput>,
) -> Result<impl IntoResponse, Error> {
	let posts = sqlx::query_as!(
		model::Post,
		r#"
    SELECT * FROM post
    WHERE user_id = $1
    ORDER BY created_at DESC
    LIMIT $2 OFFSET $3
    "#,
		session.user.id,
		paginate.size,
		paginate.size * (paginate.page - 1),
	)
	.fetch_all(&database)
	.await?;

	Ok(Json(posts))
}

pub async fn get_all_posts(
	State(database): State<Database>,
	Query(paginate): Query<PostPaginateInput>,
) -> Result<impl IntoResponse, Error> {
	let posts = sqlx::query_as!(
		model::Post,
		r#"
    SELECT * FROM post
    ORDER BY created_at DESC
    LIMIT $1 OFFSET $2
    "#,
		paginate.size,
		paginate.size * (paginate.page - 1),
	)
	.fetch_all(&database)
	.await?;

	Ok(Json(posts))
}

pub async fn get_one_post(
	State(database): State<Database>,
	Path(post_id): Path<Uuid>,
) -> Result<impl IntoResponse, Error> {
	let post = sqlx::query_as!(
		model::Post,
		r#"
    SELECT * FROM post
    WHERE id = $1
    "#,
		post_id,
	)
	.fetch_optional(&database)
	.await
	.map_err(Error::from)?;

	match post {
		Some(post) => Ok(Json(post)),
		None => Err(PostError::UnknownPost(post_id).into()),
	}
}
