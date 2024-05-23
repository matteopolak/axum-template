use uuid::Uuid;

pub const COOKIE_NAME: &str = "session";

/// Creates a session cookie with no expiry
pub fn create_cookie(session_id: Uuid) -> cookie::Cookie<'static> {
	cookie::Cookie::build((COOKIE_NAME, session_id.to_string()))
		.secure(cfg!(debug_assertions))
		.http_only(cfg!(debug_assertions))
		.path("/")
		.into()
}

/// Creates an empty session cookie used to invalidate a previous one
pub fn clear_cookie() -> cookie::Cookie<'static> {
	cookie::Cookie::build(COOKIE_NAME)
		.http_only(cfg!(debug_assertions))
		.path("/")
		.max_age(cookie::time::Duration::ZERO)
		.into()
}
