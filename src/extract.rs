use axum::{
	extract::{FromRequest, Request},
	http::header,
};
use serde::de;
use uuid::Uuid;

use crate::{error::Error, model, route::auth::AuthError, session::SESSION_COOKIE_NAME, AppState};

/// Extractor that deserializes a JSON body and validates it.
pub struct Json<T>(pub T);

#[axum::async_trait]
impl<T, S> FromRequest<S> for Json<T>
where
	T: de::DeserializeOwned + validator::Validate,
	S: Send + Sync,
{
	type Rejection = Error;

	async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
		let result = axum::extract::Json::<T>::from_request(req, state).await?.0;

		result.validate().map_err(Error::Validation)?;
		Ok(Self(result))
	}
}

/// Extracts the session and related user from the request.
///
/// If it does not exist, a [`AuthError::NoSessionCookie`] is returned.
/// If the session is invalid, a [`AuthError::InvalidSessionCookie`] is returned.
#[derive(Debug)]
pub struct Session {
	pub id: Uuid,
	pub user: model::User,
}

#[axum::async_trait]
impl FromRequest<AppState> for Session {
	type Rejection = Error;

	async fn from_request(req: Request, state: &AppState) -> Result<Self, Self::Rejection> {
		let cookie = req
			.headers()
			.get(header::COOKIE)
			.and_then(|value| value.to_str().ok())
			.unwrap_or("");

		let session_id = cookie::Cookie::split_parse(cookie)
			.filter_map(|cookie| cookie.ok())
			.find(|cookie| cookie.name() == SESSION_COOKIE_NAME)
			.ok_or(AuthError::NoSessionCookie)?;

		let session_id =
			Uuid::parse_str(session_id.value()).map_err(|_| AuthError::InvalidSessionCookie)?;

		let user = sqlx::query_as!(
			model::User,
			r#"
            SELECT * FROM "user" WHERE id = (
                SELECT user_id FROM session WHERE id = $1
            )
        "#,
			session_id
		)
		.fetch_optional(&state.database)
		.await?;

		let Some(user) = user else {
			return Err(AuthError::InvalidSessionCookie.into());
		};

		Ok(Self {
			user,
			id: session_id,
		})
	}
}
