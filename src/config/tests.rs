use super::*;
use rand::prelude::*;

const CONFIG_DIRECTORY: &str = "/tmp/japm/tests";

struct TestConfig {
    path: String,
}

impl TestConfig {
    pub fn new() -> TestConfig {
        let mut path;

        loop {
            let random_num = thread_rng().gen_range(0..100);
            path = String::from(CONFIG_DIRECTORY);
            path.push_str(&format!("/config{random_num}.json"));

            // Maybe the random config already exists
            if fs::metadata(&path).is_err() {
                break;
            }
        }

        TestConfig { path }
    }
}

impl Drop for TestConfig {
    fn drop(&mut self) {
        fs::remove_file(&self.path).expect("Could not remove test config file.");
    }
}

#[test]
fn test_default_config_created_properly() {
    let test_config = TestConfig::new();

    assert!(Config::create_default_config_if_necessary(&test_config.path).is_ok());

    let config = Config::from_file(&test_config.path).unwrap();

    assert_eq!(
        config.remotes.get("base").unwrap(),
        "https://raw.githubusercontent.com/TheAlexDev23/japm-official-packages/main/"
    );
}

#[test]
fn test_config_parsed_correctly() {
    let test_config = TestConfig::new();

    fs::write(
        &test_config.path,
        r#"
        {
            "remotes": {
                "test": "http://test.com"
            }
        }
        "#,
    )
    .expect("Could not write to test config file");

    let config = Config::from_file(&test_config.path).unwrap();

    assert_eq!(config.remotes.get("test").unwrap(), "http://test.com")
}

#[test]
fn test_incorrect_json_syntax_rejected() {
    let test_config = TestConfig::new();

    fs::write(
        &test_config.path,
        r#"
        this is invalid json syntax
        {
            "remotes": {
                "test": "http://test.com"
            ]
        }
        "#,
    )
    .expect("could not write to test config file");

    let config = Config::from_file(&test_config.path);

    assert!(config.is_err());
}

#[test]
fn test_no_remotes_field_rejected() {
    let test_config = TestConfig::new();

    fs::write(
        &test_config.path,
        r#"
        { }
        "#,
    )
    .expect("could not write to test config file.");

    let config = Config::from_file(&test_config.path);

    assert!(config.is_err());
}

#[test]
fn test_non_string_remotes_rejected() {
    let test_config = TestConfig::new();

    fs::write(
        &test_config.path,
        r#"
        {
            "remotes": [
                "test": {
                    "some_non_string_object": "http://test.com"
                }
            ]
        }
        "#,
    )
    .expect("could not write to test config file.");
    let config = Config::from_file(&test_config.path);

    assert!(config.is_err());
}
