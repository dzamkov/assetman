use assetman_image::AssetLoaderImageExt;
use image::GenericImageView;

#[test]
fn test_load_ferris() {
    let root = assetman::AssetRoot::new(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests"
    )));
    let loader = assetman::AssetLoader::new(&root, None);
    assert_eq!(loader.size_image(&"ferris.png".into()).unwrap(), [300, 200]);
    let ferris = loader.load_image(&"ferris.png".into()).unwrap();
    assert_eq!(ferris.width(), 300);
    assert_eq!(ferris.height(), 200);
    assert_eq!(ferris.get_pixel(150, 100), image::Rgba([247, 76, 0, 255]));
}
