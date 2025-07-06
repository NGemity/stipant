use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{self, Read, Seek, Write},
    path::Path,
    sync::Arc,
    thread,
};

use rzfile::file::parse_index;
use thiserror::Error;

use crate::config::AppConfig;

/// Errors that may occur within the `stipant` library during file extraction or handling.
#[derive(Error, Debug)]
enum StipantError {
    /// Raised when the expected `data.00x` file could not be found on disk.
    ///
    /// The `u8` represents the file number (1–8) that was expected.
    #[error("Invalid file path, data.00{0} not found.")]
    NoDataPath(u8),

    /// Raised when a required path is not a directory.
    #[error("Not a directory.")]
    NotADirectory,

    /// Raised when the filename does not contain a valid file extension.
    #[error("Not a file extension.")]
    NotAnExtension,

    /// Raised when no valid export directory was configured or detected.
    #[error("Invalid export directory!")]
    InvalidExportDirectory,

    /// Raised when dumping a non-existant file name which should not be possible
    #[error("Unknown error occured")]
    UnknownError,
}

/// Represents a single extracted file entry from a `.00x` archive.
///
/// This struct contains metadata needed to locate and optionally extract a file
/// from within the original game or client archive.
///
/// ### Fields
/// - `hash`: The (presumably unique) hash value used for indexing.
/// - `name`: The original or resolved file name.
/// - `file`: The index of the `.00x` file the data resides in (1–8).
/// - `found`: Marker for UI/debugging purposes to indicate if the file was matched or used.
/// - `size`: The size of the file in bytes.
/// - `offset`: The byte offset inside the `.00x` file where the file begins.
#[derive(Default, serde::Serialize)]
pub struct RZData {
    pub hash: String,
    pub name: String,
    pub file: u8,
    pub found: bool, // For visualising
    pub size: u32,
    pub offset: u32,
}

/// Central handler for managing and extracting RZData entries.
///
/// This struct acts as the main container for loaded file metadata and configuration.
/// It provides functionality for querying, exporting, and processing data files.
///
/// ### Fields
/// - `data_dir`: Base directory containing `data.00x` files.
/// - `export_dir`: Optional output directory for extracted files.
/// - `file_list`: Lookup map from file name to metadata (`RZData`).
/// - `loop_list`: Grouped list of `RZData` entries, indexed by their `.00x` file ID (1–8).
/// - `config`: Shared configuration options used for decryption, filtering, etc.
pub struct DataHandler {
    pub data_dir: String,
    pub export_dir: Option<String>,
    file_list: HashMap<String, Arc<RZData>>,
    file_groups: Arc<HashMap<u8, Vec<Arc<RZData>>>>,
    config: Arc<AppConfig>,
}

/// Implementation block for `DataHandler`, providing methods to manage,
/// extract, and interact with RZ file metadata and binary resources.
///
/// Includes utilities for:
/// - exporting files individually or in batches
/// - checking export paths
/// - retrieving metadata by name
/// - spawning parallel workers for extraction
impl DataHandler {
    /// Creates a new `DataHandler` instance by loading and parsing the `data.000` index file.
    ///
    /// This constructor performs the following:
    /// - Validates that the provided `filepath` is a directory
    /// - Ensures that `data.000` exists within that directory
    /// - Reads and decrypts the binary index data
    /// - Builds a list of `RZData` entries and organizes them into loop buckets (1..=8)
    ///
    /// # Arguments
    /// - `filepath`: Path to the directory containing `data.000` and `data.00X` files
    /// - `config`: Shared reference to an [`AppConfig`] with decryption settings
    ///
    /// # Errors
    /// Returns an [`anyhow::Error`] if:
    /// - The directory is invalid or not found
    /// - `data.000` is missing
    /// - The index file cannot be parsed
    /// - UTF-8 decoding of hashes fails
    /// - File name decoding fails (via [`rzfile::name::decode_file_name`])
    ///
    /// # Returns
    /// A fully initialized `DataHandler` with metadata and sorted file buckets.
    ///
    /// # Panics
    /// This method does **not** panic; all recoverable issues return as errors.
    ///
    /// # Example
    /// ```no_run
    /// use stipant_lib::{config::AppConfig, handler::DataHandler};
    /// use std::sync::Arc;
    ///
    /// let config = Arc::new(AppConfig::default());
    /// let handler = DataHandler::new("path/to/game/data", config).unwrap();
    /// ```
    pub fn new(filepath: &str, config: Arc<AppConfig>) -> Result<Self, anyhow::Error> {
        let path = Path::new(filepath);
        if !path.is_dir() {
            return Err(StipantError::NotADirectory.into());
        }

        let data_file = path.join("data.000");
        if !data_file.is_file() {
            return Err(StipantError::NoDataPath(0).into());
        }

        let mut init_file_groups = HashMap::new();
        for n in 1..9 {
            let _ = init_file_groups.insert(n as u8, Vec::<Arc<RZData>>::new());
        }
        // Read file into buffer
        let mut buf = Vec::new();
        File::open(data_file)?.read_to_end(&mut buf)?;

        // Parse index file
        let file = parse_index(&mut buf, config.resource_encryption_key.as_deref())?;
        let mut file_list = HashMap::new();
        let dec_table = config.dec_table.as_deref();

        for entry in file {
            let hash = String::from_utf8(entry.hash.clone())?;
            let rz_file = Arc::new(RZData {
                hash: hash.clone(),
                name: rzfile::name::decode_file_name(
                    &hash,
                    dec_table,
                    dec_table,
                    true,
                )?,
                file: rzfile::name::get_file_no(&hash),
                found: true,
                size: entry.size,
                offset: entry.offset,
            });

            if let Some(val) = init_file_groups.get_mut(&rz_file.file) {
                val.push(rz_file.clone());
            };
            let _ = file_list.insert(rz_file.name.clone(), rz_file);
        }

        for n in 1..9 {
            if let Some(val) = init_file_groups.get_mut(&n) {
                val.sort_by(|a, b| a.offset.cmp(&b.offset));
            };
        }

        Ok(Self {
            file_list,
            file_groups: Arc::new(init_file_groups),
            data_dir: filepath.to_string(),
            export_dir: None,
            config,
        })
    }

    /// Sets the export directory path.
    ///
    /// This method sets the internal export directory used by the instance for file operations
    /// such as saving or exporting data. The provided path will be stored as a `String`.
    ///
    /// # Arguments
    ///
    /// * `export_dir` - A string slice that holds the path to the export directory.
    pub fn set_export_dir(&mut self, export_dir: &str) {
        self.export_dir = Some(export_dir.to_string());
    }

    /// Returns a data entry by its file name, if it exists.
    ///
    /// Looks up the given file name in the internal file list and returns a cloned [`Arc<RZData>`]
    /// if found. Returns `None` if the file is not present.
    ///
    /// # Arguments
    ///
    /// * `file_name` - The name of the file to look up.
    ///
    /// # Returns
    ///
    /// * `Some(Arc<RZData>)` if the file exists.
    /// * `None` if the file is not found.
    pub fn get_entry_by_name(&self, file_name: &str) -> Option<Arc<RZData>> {
        self.file_list.get(file_name).cloned()
    }

    /// Returns the number of entries currently stored.
    ///
    /// This returns the number of loaded or indexed file entries.
    ///
    /// # Returns
    ///
    /// The number of file entries in the internal file list.
    pub fn len(&self) -> usize {
        self.file_list.len()
    }

    /// Checks whether the currently set export directory exists and is a directory.
    ///
    /// # Returns
    /// - `true` if `export_dir` is set and points to a valid directory
    /// - `false` if `export_dir` is `None` or does not point to a directory
    fn check_export_dir(&self) -> bool {
        self.export_dir
            .as_ref()
            .map(|p| Path::new(p).is_dir())
            .unwrap_or(false)
    }

    /// Dumps all files from the parsed RZ dataset into the configured export directory.
    ///
    /// This function spawns 8 worker threads (one per `data.00x` file) that read the
    /// relevant binary chunks and export each file using the `save_file` function.
    ///
    /// # Behavior
    /// - Skips execution if the export directory is not set or is invalid.
    /// - Each worker handles its own file (`data.001` through `data.008`) in parallel.
    /// - Thread panics will propagate via `expect`.
    ///
    /// # Panics
    /// - Panics if any of the spawned threads encounter a panic during execution.
    ///
    /// # See Also
    /// - [`spawn_worker_thread`] – internal helper for per-thread dumping logic.
    pub fn dump_all(&self) {
        if !self.check_export_dir() {
            return;
        }

        let threads: Vec<_> = (1..=8).map(|i| self.spawn_worker_thread(i)).collect();

        for handle in threads {
            handle.join().expect("Thread panicked during file dump");
        }
    }

    /// Spawns a thread responsible for dumping all files associated with a given index.
    ///
    /// This function is used internally by [`dump_all`] to handle each `data.00X` file
    /// (where `X` = index) in parallel. It attempts to open the corresponding file,
    /// retrieve its file list from `loop_list`, and export each entry to the `export_dir`.
    ///
    /// # Arguments
    /// - `index`: The numeric index (1–8) corresponding to `data.00X` file to process.
    ///
    /// # Returns
    /// - A `JoinHandle<()>` for the spawned thread.
    ///
    /// # Behavior
    /// - Skips execution if the file does not exist or cannot be opened.
    /// - Logs a message to stderr for each failed file export.
    /// - Panics if `export_dir` is `None`, which should be pre-validated by the caller.
    ///
    /// # See Also
    /// - [`dump_all`] – orchestrates multithreaded dumping
    /// - [`dump_single_file`] – handles individual file extraction logic
    fn spawn_worker_thread(&self, index: u8) -> thread::JoinHandle<()> {
        let data_dir = self.data_dir.clone();
        let loop_list = self.file_groups.clone();
        let export_dir = self.export_dir.as_ref().unwrap().clone();
        let cfg = self.config.clone();

        thread::spawn(move || {
            let path = Path::new(&data_dir).join(format!("data.00{index}"));
            if !path.is_file() {
                return;
            }

            let mut file = match OpenOptions::new().read(true).open(&path) {
                Ok(f) => f,
                Err(_) => return,
            };

            if let Some(files) = loop_list.get(&index) {
                for rz_file in files {
                    if let Err(e) =
                        Self::dump_single_file(&mut file, rz_file, &export_dir, cfg.clone())
                    {
                        eprintln!("Failed to dump {}: {}", rz_file.name, e);
                    }
                }
            }
        })
    }

    /// Dumps a single `RZData` file to the export directory.
    ///
    /// Reads the raw bytes from the given file handle at the offset and size specified in `rz_file`,
    /// then delegates saving the file to [`save_file`].
    ///
    /// # Arguments
    /// - `file`: The opened file handle (e.g., `data.00X`) positioned via offset for the entry.
    /// - `rz_file`: Metadata about the file to extract (offset, size, name).
    /// - `export_dir`: Target directory path where the file should be saved.
    /// - `config`: Shared application configuration, forwarded to the save function.
    ///
    /// # Returns
    /// - `Ok(())` on success.
    /// - `Err(anyhow::Error)` if seeking, reading, or saving fails.
    ///
    /// # Errors
    /// - Fails if seeking to the given offset or reading the expected number of bytes fails.
    /// - Fails if [`save_file`] encounters any issue (e.g., invalid path or write failure).
    ///
    /// # See Also
    /// - [`spawn_worker_thread`] – calls this for each file in a `data.00X` block.
    /// - [`save_file`] – handles the actual write logic.
    fn dump_single_file(
        file: &mut File,
        rz_file: &RZData,
        export_dir: &str,
        config: Arc<AppConfig>,
    ) -> Result<(), anyhow::Error> {
        let mut buf = vec![0u8; rz_file.size as usize];
        file.seek(std::io::SeekFrom::Start(rz_file.offset.into()))?;
        file.read_exact(&mut buf)?;

        save_file(export_dir, &rz_file.name, rz_file, &mut buf, config)
    }

    /// Dumps a single file by its name using the shared `dump_single_file` logic.
    ///
    /// Resolves the file by name, opens the corresponding `data.00X` file, and writes it
    /// to the configured export directory using the same pipeline as `dump_all`.
    ///
    /// # Arguments
    /// - `file_name`: Name of the file to extract.
    ///
    /// # Returns
    /// - `Ok(())` on success
    /// - `Err(anyhow::Error)` if any step fails, including file not found or invalid paths.
    ///
    /// # Errors
    /// - [`StipantError::InvalidExportDirectory`] if `export_dir` is unset or invalid.
    /// - [`StipantError::NoDataPath`] if the relevant `data.00X` file is missing.
    /// - I/O and save-related errors via `dump_single_file`.
    ///
    /// # See Also
    /// - [`dump_single_file`] – shared implementation used by `dump_all` and worker threads.
    pub fn dump_by_filename(&self, file_name: &str) -> Result<(), anyhow::Error> {
        if !self.check_export_dir() {
            return Err(StipantError::InvalidExportDirectory.into());
        }

        let rz_file = match self.get_entry_by_name(file_name) {
            Some(f) => f,
            None => return Err(StipantError::UnknownError.into()),
        };

        let path = Path::new(&self.data_dir).join(format!("data.00{}", rz_file.file));
        if !path.is_file() {
            return Err(StipantError::NoDataPath(rz_file.file).into());
        }

        let mut file = File::open(path)?;
        Self::dump_single_file(
            &mut file,
            &rz_file,
            self.export_dir.as_ref().unwrap(),
            self.config.clone(),
        )
    }
}

/// Saves a single `RZData` entry to the filesystem.
///
/// Handles decryption (if required), determines the appropriate subdirectory
/// by file extension, and writes the contents to disk. Files are organized in
/// subfolders named after their extension (e.g., `png/`, `txt/`, etc.).
///
/// # Arguments
/// - `export_dir`: The root directory where files should be saved.
/// - `file_name`: The name of the file (used for decryption check).
/// - `rz_file`: Metadata about the file (name, size, offset, etc.).
/// - `buf`: The in-memory content buffer to write.
/// - `config`: Shared application configuration including keys and extension filters.
///
/// # Behavior
/// - If the file is not considered "decrypted", it will be decrypted via `cipher()`
///   using the provided encryption key.
/// - If the file has no extension, [`StipantError::NotAnExtension`] is returned.
/// - Files are saved to `<export_dir>/<extension>/<filename>`.
/// - The target directory is created if it doesn't exist.
///
/// # Returns
/// - `Ok(())` on success.
/// - An `anyhow::Error` if writing or path setup fails.
///
/// # Errors
/// - Fails if the file has no extension.
/// - I/O errors during directory creation or writing are propagated.
/// - Will return `Err` on decryption or file creation failures.
fn save_file(
    export_dir: &str,
    file_name: &str,
    rz_file: &RZData,
    buf: &mut [u8],
    config: Arc<AppConfig>,
) -> Result<(), anyhow::Error> {
    if !rzfile::file::is_decrypted(file_name, config.decrypted_extensions.clone()) {
        rzfile::file::cipher(buf, config.resource_encryption_key.as_deref());
    }

    let ext = Path::new(&rz_file.name)
        .extension()
        .ok_or(StipantError::NotAnExtension)?;

    let mut new_file = Path::new(export_dir).join(ext);

    if !new_file.is_dir() {
        if let Err(e) = fs::create_dir(&new_file) {
            // Due to threads creating directories, we may have a race condition here
            // If the create_dir function returns AlreadyExists, we do not need to error
            if e.kind() != io::ErrorKind::AlreadyExists {
                return Err(e.into());
            }
        }
    }
    new_file.push(rz_file.name.clone());

    // use scope for writing
    let mut export_file = OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .open(new_file)?;

    export_file.write_all(buf)?;
    Ok(())
}
