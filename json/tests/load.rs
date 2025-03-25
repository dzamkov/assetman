use assetman::{AssetPath, Tracker};
use assetman_json::AssetPathJsonExt;

#[derive(serdere::Deserialize)]
pub struct Config {
    name: String,
    keywords: Vec<String>,
}

#[test]
fn test_load_config() {
    let root = AssetPath::new_root_fs(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests"
    )));
    let tracker = Tracker::default();
    let config = root
        .relative("config.json")
        .load_json::<Config>(&tracker)
        .unwrap();
    assert_eq!(config.name, "Test Config".to_owned());
    assert_eq!(
        config.keywords,
        vec!["test".to_owned(), "config".to_owned(), "json".to_owned()]
    );
}
