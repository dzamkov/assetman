use assetman::{AssetLoadResult, AssetLoader, AssetPath};
use serdere::{Deserialize, Outliner, Utf8Reader, Value};
use serdere_json::{TextDeserializer, TextDeserializerConfig};
use std::io::BufReader;

/// Contains JSON-loading extensions for [`AssetLoader`].
pub trait AssetLoaderJsonExt {
    /// Loads a JSON file asset using a deserializer interface.
    fn load_json_with<R>(
        &self,
        asset: &AssetPath,
        f: impl FnOnce(Value<JsonDeserializer>) -> Result<R, JsonDeserializerError>,
    ) -> AssetLoadResult<R>;

    /// Loads a JSON file asset, deserializing it into a value of type `T`.
    fn load_json<T: for<'a> Deserialize<JsonDeserializer<'a>>>(
        &self,
        asset: &AssetPath,
    ) -> AssetLoadResult<T> {
        self.load_json_with(asset, |de| de.get())
    }

    /// Loads a JSON file asset, deserializing it into a value of type `T`, using the given
    /// deserialization context.
    fn load_json_using<T: for<'a> Deserialize<JsonDeserializer<'a>, Ctx>, Ctx: ?Sized>(
        &self,
        asset: &AssetPath,
        context: &mut Ctx,
    ) -> AssetLoadResult<T> {
        self.load_json_with(asset, |de| de.get_using(context))
    }
}

impl AssetLoaderJsonExt for AssetLoader<'_> {
    fn load_json_with<R>(
        &self,
        asset: &AssetPath,
        f: impl FnOnce(Value<JsonDeserializer>) -> Result<R, JsonDeserializerError>,
    ) -> AssetLoadResult<R> {
        let mut file = self.open_file(asset)?;
        assetman::with_asset(asset, || {
            let reader = Utf8Reader::new(BufReader::<&mut dyn std::io::Read>::new(&mut file))?;
            Ok(TextDeserializer::new(
                TextDeserializerConfig::permissive(),
                reader,
            )
            .and_then(|mut deserializer| Value::with(&mut deserializer, f))?)
        })
    }
}

/// The type of JSON deserializer provided by an [`AssetLoader`].
pub type JsonDeserializer<'a> =
    TextDeserializer<Utf8Reader<std::io::BufReader<&'a mut dyn std::io::Read>>>;

/// The type of error produced by a [`JsonDeserializer`].
pub type JsonDeserializerError = <JsonDeserializer<'static> as Outliner>::Error;
