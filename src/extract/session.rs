use std::str::FromStr;

use aide::OperationInput;
use axum::{
	extract::{FromRef, FromRequestParts},
	http::{header, request},
};

use uuid::Uuid;

use crate::{
	error::RouteError,
	openapi::{SECURITY_SCHEME_API_KEY, SECURITY_SCHEME_SESSION},
	route::auth,
	session, Database,
};

pub const AUTHORIZATION_PREFIX: &str = "Bearer ";

/// A session or API key.
///
/// When fetching a user through cookie authentication,
/// this will be a [`SessionOrApiKey::Session`].
///
/// When fetching a user through API key authentication,
/// this will be a [`SessionOrApiKey::ApiKey`].
#[derive(Debug)]
pub enum SessionOrApiKey {
	Session(Uuid),
	#[allow(dead_code)]
	ApiKey(Uuid),
}

/// Extracts the session and related user from the request.
///
/// If it does not exist, a [`auth::Error::NoSessionCookie`] is returned.
/// If the session is invalid, a [`auth::Error::InvalidSessionCookie`] is returned.
///
/// ```rust
/// async fn route(session: Session) {
///   println!("{:?}", session.user);
/// }
/// ```
#[derive(Debug)]
pub struct Session {
	pub id: SessionOrApiKey,
	pub user: auth::model::User,
}

#[axum::async_trait]
impl<S> FromRequestParts<S> for Session
where
	Database: FromRef<S>,
	S: Sync + Send,
{
	type Rejection = RouteError<auth::Error>;

	/// Extracts the session from the request using a session cookie or API key.
	async fn from_request_parts(
		parts: &mut request::Parts,
		state: &S,
	) -> Result<Self, Self::Rejection> {
		let api_key = parts.headers.get(header::AUTHORIZATION);

		Ok(if let Some(api_key) = api_key {
			let slice = api_key.to_str().map_err(|_| auth::Error::InvalidApiKey)?;

			if !slice.starts_with(AUTHORIZATION_PREFIX) {
				return Err(auth::Error::InvalidApiKey.into());
			}

			let api_key = Uuid::from_str(&slice[AUTHORIZATION_PREFIX.len()..])
				.map_err(|_| auth::Error::InvalidApiKey)?;

			let database = Database::from_ref(state);
			let user = sqlx::query_as!(
				auth::model::User,
				r#"
				SELECT * FROM "user" WHERE id IN (
					SELECT user_id FROM api_key WHERE id = $1
				)
			"#,
				api_key
			)
			.fetch_optional(&database)
			.await?;

			let user = user.ok_or(auth::Error::InvalidApiKey)?;

			Session {
				user,
				id: SessionOrApiKey::ApiKey(api_key),
			}
		} else {
			let cookies = parts
				.headers
				.get_all(header::COOKIE)
				.into_iter()
				.filter_map(|value| value.to_str().ok());

			let session_id = cookies
				.flat_map(cookie::Cookie::split_parse)
				.filter_map(Result::ok)
				.find(|cookie| cookie.name() == session::COOKIE_NAME)
				.ok_or(auth::Error::NoSessionCookieOrApiKey)?;

			let session_id = Uuid::parse_str(session_id.value())
				.map_err(|_| auth::Error::InvalidSessionCookie)?;

			let database = Database::from_ref(state);
			let user = sqlx::query_as!(
				auth::model::User,
				r#"
				SELECT * FROM "user" WHERE id = (
					SELECT user_id FROM session WHERE id = $1
				)
			"#,
				session_id
			)
			.fetch_optional(&database)
			.await?;

			let user = user.ok_or(auth::Error::InvalidSessionCookie)?;

			Session {
				user,
				id: SessionOrApiKey::Session(session_id),
			}
		})
	}
}

impl OperationInput for Session {
	/// Operation input for the session extractor.
	///
	/// This adds a session cookie requirement to the `OpenAPI` operation.
	fn operation_input(_ctx: &mut aide::gen::GenContext, operation: &mut aide::openapi::Operation) {
		operation.security.extend([
			[(SECURITY_SCHEME_SESSION.to_string(), Vec::new())]
				.into_iter()
				.collect(),
			[(SECURITY_SCHEME_API_KEY.to_string(), Vec::new())]
				.into_iter()
				.collect(),
		]);
	}
}
