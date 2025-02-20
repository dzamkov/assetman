use assetman::{AssetLoader, AssetPath, AssetRoot};
use assetman_gltf::AssetLoaderGltfExt;

#[test]
fn test_load_box() {
    let root = AssetRoot::new(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests"
    )));
    let loader = AssetLoader::new(&root, None);
    for bx in ["box.gltf", "box.glb"].map(|s| loader.load_gltf(&AssetPath::absolute(s))) {
        let gltf = bx.unwrap();
        let scene = gltf.scene().unwrap();
        let node = scene.nodes().next().unwrap();
        assert_eq!(
            node.info().matrix,
            Some([1.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0])
        );
        let child = node.children().next().unwrap();
        let mesh = child.mesh().unwrap();
        let prim = mesh.primitives().next().unwrap();
        assert_eq!(prim.normal().unwrap().elements().unwrap().count(), 24);
        assert_eq!(prim.position().unwrap().elements().unwrap().count(), 24);
        prim.material().unwrap();
    }
}

#[test]
fn test_load_basket() {
    let root = AssetRoot::new(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests"
    )));
    let loader = AssetLoader::new(&root, None);
    let basket = loader
        .load_gltf(&AssetPath::absolute("basket.gltf"))
        .unwrap();
    basket.nodes_by_name("Camera").next().unwrap().camera().unwrap();
}
