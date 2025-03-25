use assetman::{AssetLoadResult, AssetPath, Tracker};
use serdere::{Deserialize, Outliner, Utf8Reader, Value};
use serdere_json::{TextDeserializer, TextDeserializerConfig};
use std::io::BufReader;

/// Contains JSON-loading extensions for [`AssetPath`].
pub trait AssetPathJsonExt {
    /// Loads a JSON file asset using a deserializer interface.
    fn load_json_with<R>(
        &self,
        tracker: &Tracker,
        f: impl FnOnce(Value<JsonDeserializer>) -> Result<R, JsonDeserializerError>,
    ) -> AssetLoadResult<R>;

    /// Loads a JSON file asset, deserializing it into a value of type `T`.
    fn load_json<T: for<'a> Deserialize<JsonDeserializer<'a>>>(
        &self,
        tracker: &Tracker,
    ) -> AssetLoadResult<T> {
        self.load_json_with(tracker, |de| de.get())
    }

    /// Loads a JSON file asset, deserializing it into a value of type `T`, using the given
    /// deserialization context.
    fn load_json_using<T: for<'a> Deserialize<JsonDeserializer<'a>, Ctx>, Ctx: ?Sized>(
        &self,
        tracker: &Tracker,
        context: &mut Ctx,
    ) -> AssetLoadResult<T> {
        self.load_json_with(tracker, |de| de.get_using(context))
    }
}

impl AssetPathJsonExt for AssetPath {
    fn load_json_with<R>(
        &self,
        tracker: &Tracker,
        f: impl FnOnce(Value<JsonDeserializer>) -> Result<R, JsonDeserializerError>,
    ) -> AssetLoadResult<R> {
        let mut file = self.open_file(tracker)?;
        assetman::with_asset(self, || {
            let reader = Utf8Reader::new(BufReader::<&mut dyn std::io::Read>::new(&mut file))?;
            Ok(
                TextDeserializer::new(TextDeserializerConfig::permissive(), reader)
                    .and_then(|mut deserializer| Value::with(&mut deserializer, f))?,
            )
        })
    }
}

/// The type of JSON deserializer provided by an [`AssetLoader`].
pub type JsonDeserializer<'a> =
    TextDeserializer<Utf8Reader<std::io::BufReader<&'a mut dyn std::io::Read>>>;

/// The type of error produced by a [`JsonDeserializer`].
pub type JsonDeserializerError = <JsonDeserializer<'static> as Outliner>::Error;
