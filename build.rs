use std::path::PathBuf;

const SCALAR_MIN_JS: &str = "https://cdn.jsdelivr.net/npm/@scalar/api-reference";

fn default_theme() -> String {
	"default".into()
}

fn default_spec_url() -> String {
	"/docs/private/api.json".into()
}

#[derive(serde::Deserialize)]
struct Config {
	scalar: Scalar,
}

#[derive(serde::Deserialize)]
struct Scalar {
	#[serde(default = "default_theme")]
	theme: String,
	#[serde(default = "default_spec_url")]
	spec_url: String,
	title: String,
}

// Example custom build script.
fn main() {
	println!("cargo:rerun-if-changed=Cargo.toml");

	// read from Cargo.toml
	let config = std::fs::read_to_string("Cargo.toml").unwrap();
	let config = toml::from_str::<Config>(&config).unwrap();

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
		</html>"#,
		title = config.scalar.title,
		theme = config.scalar.theme,
		spec_url = config.scalar.spec_url,
	);

	std::fs::write(out.join("scalar.html"), html).unwrap();
}
