use std::collections::HashMap;
use std::sync::Mutex;

use uial::react::renege;

/// Uniquely identifies a game asset or a directory of assets. Note that this is a path in a
/// "virtual" file system with platform independent behavior.
#[derive(Debug, PartialEq, Eq, Hash, Clone)]
pub struct AssetPath(String);

impl AssetPath {
    /// The [`AssetPath`] that represents the root assets directory.
    pub const fn root() -> Self {
        Self(String::new())
    }

    /// Gets the [`AssetPath`] for the directory this asset is in, or [`None`] if this is the root
    /// directory.
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

    /// Interpreting this [`AssetPath`] as a directory, constructs an [`AssetPath`] for an asset
    /// relative to it.
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

impl From<&str> for AssetPath {
    fn from(value: &str) -> Self {
        Self(value.to_owned())
    }
}

impl From<String> for AssetPath {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for AssetPath {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

/// Encapsulates the context information needed to load game assets.
#[derive(Clone)]
pub struct AssetLoader<'a> {
    root: &'a AssetRoot,
    react: Option<&'a renege::RenegeReact<'static>>,
}

impl<'a> AssetLoader<'a> {
    /// Constructs a new [`AssetLoader`].
    pub fn new(root: &'a AssetRoot, react: Option<&'a renege::RenegeReact<'static>>) -> Self {
        Self { root, react }
    }

    /// Loads a data file as raw bytes.
    pub fn load_bytes(&self, asset: &AssetPath) -> AssetLoadResult<Box<[u8]>> {
        let mut file = self.open_file(asset)?;
        with_asset(asset, || {
            let size = file.metadata().map(|m| m.len()).unwrap_or(0);
            let mut bytes = Vec::with_capacity(size as usize);
            std::io::Read::read_to_end(&mut file, &mut bytes)?;
            Ok(bytes.into_boxed_slice())
        })
    }

    /// Opens the file for the given asset.
    pub fn open_file(&self, asset: &AssetPath) -> AssetLoadResult<std::fs::File> {
        match self.root.open_file(std::path::Path::new(&*asset.0)) {
            Ok((file, token)) => {
                if let Some(react) = self.react {
                    react.depends_on(token);
                }
                Ok(file)
            }
            Err(err) => Err(AssetLoadError {
                asset: asset.clone(),
                inner: err.into(),
            }),
        }
    }

    /// Gets the names of the immediate children of the given asset directory.
    pub fn get_children(&self, asset: &AssetPath) -> AssetLoadResult<Vec<String>> {
        match self.root.get_children(std::path::Path::new(&*asset.0)) {
            Ok((children, token)) => {
                if let Some(react) = self.react {
                    react.depends_on(token);
                }
                Ok(children
                    .into_iter()
                    .map(|s| s.to_string_lossy().into_owned())
                    .collect())
            }
            Err(err) => Err(AssetLoadError {
                asset: asset.clone(),
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

/// Identifies a directory on the file system where assets are stored and watches for changes in
/// the directory.
pub struct AssetRoot {
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

    /// Opens a file given its relative path in the asset root directory. Also returns a
    /// [`renege::Token`] that is invalidated when the file is changed.
    pub fn open_file(
        &self,
        relative_path: &std::path::Path,
    ) -> std::io::Result<(std::fs::File, renege::Token)> {
        let full_path = self.path.join(relative_path);
        let file = std::fs::File::open(&full_path)?;
        let token = if let Some(watcher) = &self.watcher {
            use std::collections::hash_map::Entry::*;
            let mut paths = watcher.paths.lock().unwrap();
            match paths.entry(full_path) {
                Occupied(entry) => entry.get().token(),
                Vacant(entry) => entry.insert(renege::Condition::new()).token(),
            }
        } else {
            renege::Token::always()
        };
        Ok((file, token))
    }

    /// Gets the names of the immediate children of a given directory in the asset root directory.
    /// Also returns a [`renege::Token`] that is invalidated when the children of the directory
    /// change.
    pub fn get_children(
        &self,
        relative_path: &std::path::Path,
    ) -> std::io::Result<(Vec<std::ffi::OsString>, renege::Token)> {
        let full_path = self.path.join(relative_path);
        let children = std::fs::read_dir(&full_path)?
            .map(|entry| entry.map(|entry| entry.file_name()))
            .collect::<Result<_, _>>()?;
        let token = if let Some(watcher) = &self.watcher {
            use std::collections::hash_map::Entry::*;
            let mut paths = watcher.paths.lock().unwrap();
            match paths.entry(full_path) {
                Occupied(entry) => entry.get().token(),
                Vacant(entry) => entry.insert(renege::Condition::new()).token(),
            }
        } else {
            renege::Token::always()
        };
        Ok((children, token))
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