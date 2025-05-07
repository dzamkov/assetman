use assetman::{AssetPath, Tracker};
use assetman_shader::AssetPathShaderExt;

#[test]
fn test_load_minimal() {
    let root = AssetPath::new_root_fs(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests"
    )));
    let tracker = Tracker::default();
    let (device, _) = pollster::block_on(get_device());
    root.relative("minimal.wgsl")
        .load_shader_wgpu(&tracker, &device)
        .unwrap();
}

#[test]
fn test_load_error() {
    let root = AssetPath::new_root_fs(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests"
    )));
    let tracker = Tracker::default();
    let (device, _) = pollster::block_on(get_device());
    let err = root
        .relative("error.wgsl")
        .load_shader_wgpu(&tracker, &device)
        .err()
        .unwrap();
    assert_eq!(err.asset, root.relative("error.wgsl"));
}

/// Gets a [`wgpu::Device`] for testing.
async fn get_device() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .unwrap();
    adapter
        .request_device(&wgpu::DeviceDescriptor::default())
        .await
        .unwrap()
}
