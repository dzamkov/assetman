use assetman::{AssetLoadResult, AssetPath, Tracker};
use std::borrow::Cow;

/// Contains shader-related extensions for [`AssetPath`].
pub trait AssetPathShaderExt {
    /// Loads and compiles a shader.
    fn load_shader_wgpu(
        &self,
        tracker: &Tracker,
        device: &wgpu::Device,
    ) -> AssetLoadResult<wgpu::ShaderModule>;
}

impl AssetPathShaderExt for AssetPath {
    fn load_shader_wgpu(
        &self,
        tracker: &Tracker,
        device: &wgpu::Device,
    ) -> AssetLoadResult<wgpu::ShaderModule> {
        let mut file = self.open_file(tracker)?;
        assetman::with_asset(self, || {
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
