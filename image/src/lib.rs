use assetman::{AssetLoadResult, AssetLoader, AssetPath};
use std::io::BufReader;

pub use image::*;

/// Contains image-loading extensions for [`AssetLoader`].
pub trait AssetLoaderImageExt {
    /// Loads an image.
    fn load_image(&self, asset: &AssetPath) -> AssetLoadResult<DynamicImage>;

    /// Gets the size of an image at the given path.
    fn size_image(&self, asset: &AssetPath) -> AssetLoadResult<[u32; 2]>;
}

impl AssetLoaderImageExt for AssetLoader<'_> {
    fn load_image(&self, asset: &AssetPath) -> AssetLoadResult<DynamicImage> {
        let file = self.open_file(asset)?;
        let reader = BufReader::new(file);
        assetman::with_asset(asset, || {
            Ok(load(
                reader,
                image_format_from_extension(asset.extension())?,
            )?)
        })
    }

    fn size_image(&self, asset: &AssetPath) -> AssetLoadResult<[u32; 2]> {
        let file = self.open_file(asset)?;
        let reader = BufReader::new(file);
        assetman::with_asset(asset, || {
            let format = image_format_from_extension(asset.extension())?;
            let (width, height) = match format {
                ImageFormat::Png => codecs::png::PngDecoder::new(reader)?.dimensions(),
                _ => todo!(),
            };
            Ok([width, height])
        })
    }
}

/// Gets the [`ImageFormat`] for the given file extension, or returns an error if the format
/// is not recognized.
fn image_format_from_extension(extension: Option<&str>) -> ImageResult<ImageFormat> {
    extension
        .and_then(ImageFormat::from_extension)
        .ok_or_else(|| ImageError::Unsupported(error::ImageFormatHint::Unknown.into()))
}
