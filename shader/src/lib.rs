use assetman::{AssetLoadResult, AssetLoader, AssetPath};
use std::borrow::Cow;

/// Contains shader-related extensions for [`AssetLoader`].
pub trait AssetLoaderShaderExt {
    /// Loads and compiles a shader.
    fn load_shader_wgpu(
        &self,
        asset: &AssetPath,
        device: &wgpu::Device,
    ) -> AssetLoadResult<wgpu::ShaderModule>;
}

impl AssetLoaderShaderExt for AssetLoader<'_> {
    fn load_shader_wgpu(
        &self,
        asset: &AssetPath,
        device: &wgpu::Device,
    ) -> AssetLoadResult<wgpu::ShaderModule> {
        let mut file = self.open_file(asset)?;
        assetman::with_asset(asset, || {
            let mut source = String::new();
            std::io::Read::read_to_string(&mut file, &mut source)?;
            device.push_error_scope(wgpu::ErrorFilter::Validation);
            let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: None,
                source: wgpu::ShaderSource::Wgsl(Cow::Owned(source)),
            });
            let err = pollster::block_on(device.pop_error_scope());
            if let Some(wgpu::Error::Validation { description, .. }) = err {
                Err(Box::new(ShaderCompileError { description }))
            } else {
                Ok(module)
            }
        })
    }
}

/// An error that occurs during an attempt to load a shader with compiler errors.
#[derive(thiserror::Error, Debug)]
#[error("failed to compile shader: {description}")]
struct ShaderCompileError {
    description: String,
}
