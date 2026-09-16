use circleci_tui_rs::config::Config;
use std::{env, fs, path::PathBuf, time::SystemTime};

struct TestEnvironment {
    original_dir: PathBuf,
    original_token: Option<String>,
    original_slug: Option<String>,
    temp_dir: PathBuf,
}

impl Drop for TestEnvironment {
    fn drop(&mut self) {
        env::set_current_dir(&self.original_dir).expect("restore working directory");
        restore_var("CIRCLECI_TOKEN", self.original_token.take());
        restore_var("PROJECT_SLUG", self.original_slug.take());
        fs::remove_dir_all(&self.temp_dir).expect("remove temporary directory");
    }
}

fn restore_var(name: &str, value: Option<String>) {
    match value {
        Some(value) => env::set_var(name, value),
        None => env::remove_var(name),
    }
}

#[test]
fn env_local_overrides_env_but_not_process_environment() {
    let unique = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .expect("system clock after Unix epoch")
        .as_nanos();
    let temp_dir = env::temp_dir().join(format!(
        "circleci-tui-config-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir(&temp_dir).expect("create temporary directory");

    let _guard = TestEnvironment {
        original_dir: env::current_dir().expect("read working directory"),
        original_token: env::var("CIRCLECI_TOKEN").ok(),
        original_slug: env::var("PROJECT_SLUG").ok(),
        temp_dir: temp_dir.clone(),
    };

    fs::write(
        temp_dir.join(".env"),
        "CIRCLECI_TOKEN=base-token\nPROJECT_SLUG=gh/base/project\n",
    )
    .expect("write .env");
    fs::write(temp_dir.join(".env.local"), "CIRCLECI_TOKEN=local-token\n")
        .expect("write .env.local");

    env::set_current_dir(&temp_dir).expect("switch to temporary directory");
    env::remove_var("CIRCLECI_TOKEN");
    env::remove_var("PROJECT_SLUG");

    let config = Config::load().expect("load dotenv configuration");
    assert_eq!(config.circle_token, "local-token");
    assert_eq!(config.project_slug, "gh/base/project");

    env::set_var("CIRCLECI_TOKEN", "process-token");
    let config = Config::load().expect("load process configuration");
    assert_eq!(config.circle_token, "process-token");
}
