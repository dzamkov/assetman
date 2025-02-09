use assetman::{AssetLoader, AssetPath, AssetRoot};
use assetman_json::AssetLoaderJsonExt;

#[derive(serdere::Deserialize)]
pub struct Config {
    name: String,
    keywords: Vec<String>,
}

#[test]
fn test_load_config() {
    let root = AssetRoot::new(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests"
    )));
    let loader = AssetLoader::new(&root, None);
    let config = loader.load_json::<Config>(&AssetPath::absolute("config.json")).unwrap();
    assert_eq!(config.name, "Test Config".to_owned());
    assert_eq!(
        config.keywords,
        vec!["test".to_owned(), "config".to_owned(), "json".to_owned()]
    );
}
