use std::path::PathBuf;

const SCALAR_MIN_JS: &str = "https://cdn.jsdelivr.net/npm/@scalar/api-reference";

// Example custom build script.
fn main() {
	println!("cargo:rerun-if-env-changed=SCALAR_THEME");
	println!("cargo:rerun-if-env-changed=SCALAR_SPEC_URL");
	println!("cargo:rerun-if-env-changed=SCALAR_TITLE");

	let theme = std::env::var("SCALAR_THEME").unwrap_or_else(|_| "default".into());
	let spec_url =
		std::env::var("SCALAR_SPEC_URL").unwrap_or_else(|_| "/docs/private/api.json".into());
	let title = std::env::var("SCALAR_TITLE")
		.or_else(|_| std::env::var("CARGO_PKG_NAME"))
		.unwrap();

	let out = PathBuf::from(std::env::var("OUT_DIR").unwrap());

	let js = ureq::get(SCALAR_MIN_JS)
		.call()
		.unwrap()
		.into_string()
		.unwrap();

	let html = format!(
		r#"<!DOCTYPE html>
		<html>
			<head>
				<title>{title}</title>
				<meta charset="utf-8" />
				<meta
					name="viewport"
					content="width=device-width, initial-scale=1" />
				<style>
					body {{
						margin: 0;
					}}
				</style>
			</head>
			<body>
				<script
					id="api-reference"></script>
				<script>
					var configuration = {{
						theme: '{theme}',
						spec: {{
							url: '{spec_url}'
						}}
					}}

					var apiReference = document.getElementById('api-reference')
					apiReference.dataset.configuration = JSON.stringify(configuration)
				</script>
				<script>
					{js}
				</script>
			</body>
		</html>"#
	);

	std::fs::write(out.join("scalar.html"), html).unwrap();
}
