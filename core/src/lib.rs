use std::collections::HashMap;
use std::sync::{Arc, Mutex};

/// Represents a game asset or a directory of assets.
///
/// This consists of two components:
///  * A reference to a virtual file system which contains all assets that are reachable from
///    this [`AssetPath`].
///  * A path within that virtual file system which identifies a specific asset or directory of
///    assets.
#[derive(Clone)]
pub struct AssetPath {
    root: Arc<AssetRoot>,
    inner: AssetInnerPath,
}

impl PartialEq for AssetPath {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.root.as_ref(), other.root.as_ref()) && self.inner == other.inner
    }
}

impl Eq for AssetPath {}

impl std::hash::Hash for AssetPath {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        std::ptr::hash(self.root.as_ref(), state);
        self.inner.hash(state);
    }
}

impl std::fmt::Debug for AssetPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AssetPath")
            .field("inner", &self.inner)
            .finish_non_exhaustive()
    }
}

impl std::fmt::Display for AssetPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.inner.0, f)
    }
}

impl AssetPath {
    /// Constructs a "root" [`AssetPath`] from the given file system path.
    ///
    /// As a root, the returned [`AssetPath`] does not allow access to any files or directories
    /// outside of the given path. For best performance, this should be called once per asset
    /// source, and all inner [`AssetPath`]s should be derived from the result of that call.
    pub fn new_root_fs(path: &std::path::Path) -> Self {
        Self {
            root: Arc::new(AssetRoot::new(path)),
            inner: AssetInnerPath::root(),
        }
    }

    /// Gets the [`AssetPath`] for the directory this asset is in, or [`None`] if this is the root
    /// directory.
    pub fn parent(&self) -> Option<Self> {
        Some(Self {
            root: self.root.clone(),
            inner: self.inner.parent()?,
        })
    }

    /// Interpreting this [`AssetPath`] as a directory, constructs an [`AssetPath`] for an asset
    /// relative to it.
    pub fn relative(&self, path: &str) -> Self {
        Self {
            root: self.root.clone(),
            inner: self.inner.relative(path),
        }
    }

    /// Gets the file extension of this asset, or [`None`] if not present.
    pub fn extension(&self) -> Option<&str> {
        self.inner.extension()
    }
}

/// Identifies a directory on the file system where assets are stored and watches for changes in
/// the directory.
struct AssetRoot {
    /// The path to the directory.
    path: std::path::PathBuf,

    /// The file system watcher used to detect changes in the directory, or [`None`] if watching is
    /// disabled or if we failed to create a watcher.
    watcher: Option<AssetRootWatcher>,
}

/// Provides information about the file system watcher used to detect changes in an asset root
/// directory.
struct AssetRootWatcher {
    /// The underlying [`notify`] watcher used to detect changes in the directory.
    #[allow(unused)] // Need to hold to prevent dropping
    source: notify::RecommendedWatcher,

    /// A mapping from files and directories that are being watched to the [`renege::Condition`]
    /// that must be invalidated when the file or directory contents are changed.
    paths: std::sync::Arc<Mutex<HashMap<std::path::PathBuf, renege::Condition>>>,
}

impl AssetRoot {
    /// Creates a new [`AssetRoot`] for the given directory.
    pub fn new(path: &std::path::Path) -> Self {
        let path = path.canonicalize().unwrap();
        let watcher = AssetRootWatcher::new(&path)
            .map_err(|err| {
                log::error!(
                    target: "assetman",
                    "Failed to create file system watcher for asset root {:?}: {}",
                    path,
                    err
                );
            })
            .ok();
        Self { path, watcher }
    }

    /// Opens a file given its relative path in the asset root directory.
    pub fn open_file(
        &self,
        tracker: &Tracker,
        relative_path: &std::path::Path,
    ) -> std::io::Result<std::fs::File> {
        let full_path = self.path.join(relative_path);
        let file = std::fs::File::open(&full_path)?;
        if let Some(watcher) = &self.watcher {
            use std::collections::hash_map::Entry::*;
            let mut paths = watcher.paths.lock().unwrap();
            let token = match paths.entry(full_path) {
                Occupied(entry) => entry.get().token(),
                Vacant(entry) => entry.insert(renege::Condition::new()).token(),
            };
            tracker.set(tracker.get() & token);
        };
        Ok(file)
    }

    /// Ensures that the given [`Tracker`] is notified when the file at the given relative path
    /// is modified.
    pub fn track_file(
        &self,
        tracker: &Tracker,
        relative_path: &std::path::Path,
    ) {
        let full_path = self.path.join(relative_path);
        if let Some(watcher) = &self.watcher {
            use std::collections::hash_map::Entry::*;
            let mut paths = watcher.paths.lock().unwrap();
            let token = match paths.entry(full_path) {
                Occupied(entry) => entry.get().token(),
                Vacant(entry) => entry.insert(renege::Condition::new()).token(),
            };
            tracker.set(tracker.get() & token);
        };
    }

    /// Gets the names of the immediate children of a given directory in the asset root directory.
    pub fn get_children(
        &self,
        tracker: &Tracker,
        relative_path: &std::path::Path,
    ) -> std::io::Result<Vec<std::ffi::OsString>> {
        let full_path = self.path.join(relative_path);
        let children = std::fs::read_dir(&full_path)?
            .map(|entry| entry.map(|entry| entry.file_name()))
            .collect::<Result<_, _>>()?;
        if let Some(watcher) = &self.watcher {
            use std::collections::hash_map::Entry::*;
            let mut paths = watcher.paths.lock().unwrap();
            let token = match paths.entry(full_path) {
                Occupied(entry) => entry.get().token(),
                Vacant(entry) => entry.insert(renege::Condition::new()).token(),
            };
            tracker.set(tracker.get() & token);
        };
        Ok(children)
    }
}

impl AssetRootWatcher {
    /// Attempts to create a new [`AssetRootWatcher`] for the given directory.
    pub fn new(path: &std::path::Path) -> notify::Result<Self> {
        use notify::Watcher;
        let paths = std::sync::Arc::new(Mutex::new(HashMap::new()));
        let mut source = notify::RecommendedWatcher::new(
            {
                let paths = paths.clone();
                move |res: notify::Result<notify::Event>| {
                    if let Ok(event) = res {
                        let mut paths = paths.lock().unwrap();
                        for path in event.paths {
                            paths.remove(&path);
                        }
                    }
                }
            },
            Default::default(),
        )?;
        source.watch(path, notify::RecursiveMode::Recursive)?;
        Ok(Self { source, paths })
    }
}

/// The path component of an [`AssetPath`].
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
struct AssetInnerPath(String);

impl AssetInnerPath {
    /// The [`AssetInnerPath`] that represents the root directory.
    pub const fn root() -> Self {
        Self(String::new())
    }

    /// Gets the [`AssetInnerPath`] for the directory this asset is in, or [`None`] if this is the
    /// root directory.
    pub fn parent(&self) -> Option<Self> {
        let path = &self.0;
        if path.is_empty() {
            None
        } else {
            Some(
                self.0
                    .rfind('/')
                    .map(|i| Self(path[..i].to_owned()))
                    .unwrap_or(Self::root()),
            )
        }
    }

    /// Interpreting this [`AssetInnerPath`] as a directory, constructs an [`AssetInnerPath`] for
    /// an asset relative to it.
    pub fn relative(&self, path: &str) -> Self {
        let mut res = self.0.clone();
        for part in path.split('/') {
            match part {
                "." => {}
                ".." => {
                    if let Some(pos) = res.rfind('/') {
                        res.truncate(pos);
                    } else {
                        res.clear();
                    }
                }
                "~" => {
                    res.clear();
                }
                part => {
                    if !res.is_empty() {
                        res.push('/');
                    }
                    res.push_str(part);
                }
            }
        }
        Self(res)
    }

    /// Gets the file extension of this asset, or [`None`] if not present.
    pub fn extension(&self) -> Option<&str> {
        if let Some(pos) = self.0.rfind('.') {
            Some(&self.0[pos + 1..])
        } else {
            None
        }
    }
}

impl From<String> for AssetInnerPath {
    fn from(value: String) -> Self {
        Self(value)
    }
}

/// Tracks when an observation is invalidated, which is used to support hot reloading of assets.
pub type Tracker = std::cell::Cell<renege::Token>;

impl AssetPath {
    /// Loads a data file as raw bytes.
    pub fn load_bytes(&self, tracker: &Tracker) -> AssetLoadResult<Box<[u8]>> {
        let mut file = self.open_file(tracker)?;
        with_asset(self, || {
            let size = file.metadata().map(|m| m.len()).unwrap_or(0);
            let mut bytes = Vec::with_capacity(size as usize);
            std::io::Read::read_to_end(&mut file, &mut bytes)?;
            Ok(bytes.into_boxed_slice())
        })
    }

    /// Opens the file for the given asset.
    pub fn open_file(&self, tracker: &Tracker) -> AssetLoadResult<std::fs::File> {
        match self
            .root
            .open_file(tracker, std::path::Path::new(&*self.inner.0))
        {
            Ok(file) => Ok(file),
            Err(err) => Err(AssetLoadError {
                asset: self.clone(),
                inner: err.into(),
            }),
        }
    }

    /// Ensures that the given [`Tracker`] is notified when this asset is modified.
    pub fn track(&self, tracker: &Tracker) {
        self.root.track_file(tracker, std::path::Path::new(&*self.inner.0));
    }

    /// Gets the names of the immediate children of the given asset directory.
    pub fn get_children(&self, tracker: &Tracker) -> AssetLoadResult<Vec<String>> {
        match self
            .root
            .get_children(tracker, std::path::Path::new(&*self.inner.0))
        {
            Ok(children) => Ok(children
                .into_iter()
                .map(|s| s.to_string_lossy().into_owned())
                .collect()),
            Err(err) => Err(AssetLoadError {
                asset: self.clone(),
                inner: err.into(),
            }),
        }
    }
}

/// Executes an inner closure and tags errors that occur with a particular asset path.
pub fn with_asset<T>(
    asset: &AssetPath,
    inner: impl FnOnce() -> Result<T, AssetLoadInnerError>,
) -> AssetLoadResult<T> {
    inner().map_err(|e| AssetLoadError {
        asset: asset.clone(),
        inner: e,
    })
}

/// The result of loading an asset.
pub type AssetLoadResult<T> = Result<T, AssetLoadError>;

/// Describes an error that can occur while loading an asset.
#[derive(thiserror::Error, Debug)]
#[error("failed to load asset {asset}: {inner}")]
pub struct AssetLoadError {
    /// The path to the asset that we attempted to load.
    pub asset: AssetPath,

    /// Describes the error that occurred.
    #[source]
    pub inner: AssetLoadInnerError,
}

/// The inner content of an [`AssetLoadError`], which doesn't specify the asset path.
pub type AssetLoadInnerError = Box<dyn std::error::Error>;
