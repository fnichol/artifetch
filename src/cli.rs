use std::env;
use std::path::{Path, PathBuf};
use structopt::clap::AppSettings;
use structopt::StructOpt;

const AUTHOR: &str = concat!(env!("CARGO_PKG_AUTHORS"), "\n\n");

lazy_static::lazy_static! {
    static ref DEFAULT_CONFIG: PathBuf = default_config();
}

/// Here's my app about
///
/// And the long about.
#[derive(Debug, StructOpt)]
#[structopt(raw(
    setting = "AppSettings::UnifiedHelpMessage",
    max_term_width = "100",
    author = "AUTHOR",
    version = "BuildInfo::version_short()",
    long_version = "BuildInfo::version_long()",
))]
pub(crate) struct Args {
    /// Path to configuration.
    #[structopt(
        short = "c",
        long = "config",
        rename_all = "screaming_snake_case",
        raw(default_value_os = "DEFAULT_CONFIG.as_path().as_os_str()")
    )]
    config: PathBuf,
}

impl Args {
    pub(crate) fn config_path(&self) -> Option<&Path> {
        if self.config.is_file() {
            Some(self.config.as_path())
        } else {
            None
        }
    }
}
fn default_config() -> PathBuf {
    env::var("XDG_CONFIG_HOME")
        .as_ref()
        .ok()
        .and_then(|pstr| {
            let path = Path::new(pstr);
            if path.is_absolute() {
                Some(path.to_path_buf())
            } else {
                None
            }
        })
        .or_else(|| dirs_sys::home_dir().map(|path| path.join(".config")))
        .map(|path| path.join(env!("CARGO_PKG_NAME")).join("config.json"))
        .unwrap_or_else(|| PathBuf::from("/unknown"))
}

/// Build time metadata
struct BuildInfo;

impl BuildInfo {
    fn version_short() -> &'static str {
        include_str!(concat!(env!("OUT_DIR"), "/version_short.txt"))
    }

    fn version_long() -> &'static str {
        include_str!(concat!(env!("OUT_DIR"), "/version_long.txt"))
    }
}
