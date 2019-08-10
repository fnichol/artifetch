use ghrr::app::{self, config::Config};
use std::io;
use std::process;

fn main() {
    if let Err(err) = try_main() {
        eprintln!("error: {}", err);
        process::exit(1);
    }
}

fn try_main() -> io::Result<()> {
    std::env::set_var("RUST_LOG", "actix_server=info,actix_web=info,ghrr=info");
    env_logger::init();

    app::run(stub_config())
}

// TODO: remove
fn stub_config() -> Config {
    use ghrr::{app::config::RegistryConfig, Repo};
    use std::collections::HashMap;

    let oauth_token = std::env::var("GITHUB_TOKEN").expect("Must set GITHUB_TOKEN");

    let bind_addr = "127.0.0.1:8080".parse().expect("addr should parse");
    let mut registry = HashMap::new();
    registry.insert(
        "github.com".to_string(),
        RegistryConfig::GitHub {
            oauth_token,
            repos: vec![
                // Repo::new("fnichol", "mtoc"),
                // Repo::new("fnichol", "versio"),
                // Repo::new("fnichol", "libsh"),
                // Repo::new("fnichol", "names"),
                Repo::new("fnichol", "testr"),
            ],
        },
    );

    Config {
        bind_addr,
        registry,
    }
}
