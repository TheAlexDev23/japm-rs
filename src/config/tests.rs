use super::*;

const CONFIG_PATH: &str = "/tmp/japm/tests/config.json";

#[test]
fn test_default_config_created_properly() {
    if fs::metadata(CONFIG_PATH).is_ok() {
        fs::remove_file(CONFIG_PATH).expect("Could not remove test file");
    }

    assert!(Config::create_default_config_if_necessary(CONFIG_PATH).is_ok());

    assert!(fs::metadata(CONFIG_PATH).is_ok());

    assert!(Config::write_default_config(CONFIG_PATH).is_ok());

    let config = Config::from_file(CONFIG_PATH);

    assert!(config.is_ok());

    let config = config.unwrap();

    assert_eq!(
        config.remotes.get("base").unwrap(),
        "https://raw.githubusercontent.com/TheAlexDev23/japm-official-packages/main/"
    );

    fs::remove_file(CONFIG_PATH).expect("Could not cleanup test config");
}

#[test]
fn test_config_parsed_correctly() {
    let config = r#"
{
    "remotes": {
        "test": "http://test.com"
    }
}
"#;

    let config = Config::from_json(&config);
    assert!(config.is_ok());

    assert_eq!(config.unwrap().remotes.get("test").unwrap(), "http://test.com")
}

#[test]
fn test_incorrect_json_syntax_rejected() {
    let config = r#"
this is invalid json syntax
{
    "remotes": {
        "test": "http://test.com"
    ]
}
"#;

    let config = Config::from_json(&config);

    assert!(config.is_err());
    assert!(matches!(config, Err(Error::Json(_))));
}

#[test]
fn test_no_remotes_field_rejected() {
    let config = "{ }";

    let config = Config::from_json(&config);

    assert!(config.is_err());
    assert!(matches!(config, Err(Error::Syntax(_))));
}

#[test]
fn test_non_string_remotes_rejected() {
    let config = r#"
{
    "remotes": {
        "key with non strign value": {
            "some_non_string_object": "http://test.com"
        }
    }
}
"#;

    let config = Config::from_json(&config);

    assert!(config.is_err());
    assert!(matches!(config, Err(Error::Syntax(_))));
}
