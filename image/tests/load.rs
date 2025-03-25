use assetman::{AssetPath, Tracker};
use assetman_image::AssetPathImageExt;
use image::GenericImageView;

#[test]
fn test_load_ferris() {
    let root = AssetPath::new_root_fs(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests"
    )));
    let tracker = Tracker::default();
    let ferris = root.relative("ferris.png");
    assert_eq!(ferris.size_image(&tracker).unwrap(), [300, 200]);
    let ferris = ferris.load_image(&tracker).unwrap();
    assert_eq!(ferris.width(), 300);
    assert_eq!(ferris.height(), 200);
    assert_eq!(ferris.get_pixel(150, 100), image::Rgba([247, 76, 0, 255]));
}
