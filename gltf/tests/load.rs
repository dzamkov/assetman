use assetman::{AssetPath, Tracker};
use assetman_gltf::AssetPathGltfExt;

#[test]
fn test_load_box() {
    let root = AssetPath::new_root_fs(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests"
    )));
    let tracker = Tracker::default();
    for bx in ["box.gltf", "box.glb"].map(|s| root.relative(s).load_gltf(&tracker)) {
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
    let root = AssetPath::new_root_fs(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests"
    )));
    let tracker = Tracker::default();
    let basket = root.relative("basket.gltf").load_gltf(&tracker).unwrap();
    basket
        .nodes_by_name("Camera")
        .next()
        .unwrap()
        .camera()
        .unwrap();
}
