use uuid::Uuid;

pub const SESSION_COOKIE_NAME: &str = "sessionid";

pub fn create_cookie(session_id: Uuid) -> cookie::Cookie<'static> {
	cookie::Cookie::build((SESSION_COOKIE_NAME, session_id.to_string()))
		.http_only(true)
		.into()
}

pub fn clear_cookie() -> cookie::Cookie<'static> {
	cookie::Cookie::build(SESSION_COOKIE_NAME)
		.http_only(true)
		.max_age(cookie::time::Duration::ZERO)
		.into()
}
