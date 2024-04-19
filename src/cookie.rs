use uuid::Uuid;

pub const SESSION_COOKIE: &str = "sessionid";

pub fn session(sessionid: Uuid) -> cookie::Cookie<'static> {
	let cookie = cookie::Cookie::new(SESSION_COOKIE, sessionid.to_string());

	cookie::Cookie::build(cookie).http_only(true).into()
}
