use assetman::{AssetLoader, AssetPath, AssetRoot};
use assetman_shader::AssetLoaderShaderExt;

#[test]
fn test_load_minimal() {
    let root = AssetRoot::new(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests"
    )));
    let loader = AssetLoader::new(&root, None);
    let (device, _) = pollster::block_on(get_device());
    loader
        .load_shader_wgpu(&AssetPath::absolute("minimal.wgsl"), &device)
        .unwrap();
}

#[test]
fn test_load_error() {
    let root = AssetRoot::new(std::path::Path::new(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests"
    )));
    let loader = AssetLoader::new(&root, None);
    let (device, _) = pollster::block_on(get_device());
    let err = loader
        .load_shader_wgpu(&AssetPath::absolute("error.wgsl"), &device)
        .err()
        .unwrap();
    assert_eq!(err.asset, AssetPath::absolute("error.wgsl"));
}

/// Gets a [`wgpu::Device`] for testing.
async fn get_device() -> (wgpu::Device, wgpu::Queue) {
    let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions::default())
        .await
        .unwrap();
    adapter
        .request_device(&wgpu::DeviceDescriptor::default(), None)
        .await
        .unwrap()
}
