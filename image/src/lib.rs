use assetman::{AssetLoadResult, AssetPath, Tracker};
use std::io::BufReader;

pub use image::*;

/// Contains image-loading extensions for [`AssetPath`].
pub trait AssetPathImageExt {
    /// Loads an image.
    fn load_image(&self, tracker: &Tracker) -> AssetLoadResult<DynamicImage>;

    /// Gets the size of an image at the given path.
    fn size_image(&self, tracker: &Tracker) -> AssetLoadResult<[u32; 2]>;
}

impl AssetPathImageExt for AssetPath {
    fn load_image(&self, tracker: &Tracker) -> AssetLoadResult<DynamicImage> {
        let file = self.open_file(tracker)?;
        let reader = BufReader::new(file);
        assetman::with_asset(self, || {
            Ok(load(
                reader,
                image_format_from_extension(self.extension())?,
            )?)
        })
    }

    fn size_image(&self, tracker: &Tracker) -> AssetLoadResult<[u32; 2]> {
        let file = self.open_file(tracker)?;
        let reader = BufReader::new(file);
        assetman::with_asset(self, || {
            let format = image_format_from_extension(self.extension())?;
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
