use config::{Config, ConfigError, Environment, File};
use serde_derive::Deserialize;
use base64::{engine::general_purpose, Engine as _};

/// Configuration file structure used for initializing `AppConfig`.
///
/// This struct is intended to be deserialized from a configuration file (YAML).
/// All values are expected as `String`, even if they later require parsing or transformation.
#[derive(Debug, Default, Deserialize)]
struct ConfigFile {
    /// Key used for encoding/decrypting resource files.
    ///
    /// This key is typically required when files are stored in an encrypted format
    /// and must be decrypted during extraction or analysis.
    pub resource_encode_key: String,

    /// Decryption table used for resolving hashed filenames.
    ///
    /// This table defines how hashes are mapped back to original file names.
    /// Usually provided as a base64 or hex-encoded string.
    pub dec_table: String,

    /// Reference table used during decoding (or encoding) of filenames.
    ///
    /// May be identical to `dec_table`, depending on implementation.
    /// Used for character lookups and transformations.
    pub ref_table: String,

    /// A comma-separated list of file extensions considered "decrypted".
    ///
    /// Files with these extensions will not be passed through the decryption step.
    /// Example: `"png;jpg;xml"`
    pub decrypted_extensions: String,
}

/// Implementation of the `ConfigFile` structure, which handles loading configuration
/// values from optional YAML files and environment variables.
///
/// The expected structure includes fields like encryption keys, decryption tables,
/// and file extension filters used in the stipant tool.
///
/// This implementation is intended for internal loading of raw configuration values.
/// It can be later converted into a normalized `AppConfig` used throughout the application.
impl ConfigFile {
    /// Loads and deserializes a configuration file into `ConfigFile`.
    ///
    /// This method uses the [`config`] crate to merge configuration sources in the following order:
    /// 1. A YAML file located at `config/default.yaml` (optional)
    /// 2. Environment variables prefixed with `STIPANT_`
    ///
    /// The configuration is expected to match the `ConfigFile` structure.
    ///
    /// # Returns
    /// - `Ok(ConfigFile)` on successful deserialization.
    /// - `Err(ConfigError)` if loading or parsing the configuration fails.
    pub fn new() -> Result<Self, ConfigError> {
        let s = Config::builder()
            // Start off by merging in the "default" configuration file
            .add_source(
                File::with_name("config/default")
                .format(config::FileFormat::Yaml)
                .required(false))
            .add_source(Environment::with_prefix("stipant"))
            .build()?;

        // You can deserialize (and thus freeze) the entire configuration as
        s.try_deserialize()
    }
}

/// Runtime configuration structure for the `stipant` library.
///
/// This struct contains all decoded and processed configuration values,
/// such as decryption keys, lookup tables, and lists of extensions that
/// should **not** be decrypted during file extraction.
///
/// The fields are all optional to support partial or missing configuration inputs.
#[derive(Debug, Default)]
pub struct AppConfig {
    pub resource_encryption_key: Option<Vec<u8>>,
    pub dec_table: Option<Vec<u8>>,
    pub ref_table: Option<Vec<u8>>,
    pub decrypted_extensions: Option<Vec<String>>
}

impl AppConfig {
    /// Constructs a new `AppConfig` by loading and transforming raw configuration.
    ///
    /// Internally, this method:
    /// - Loads the configuration from `ConfigFile::new()` (YAML + environment).
    /// - Decodes all Base64-encoded fields into binary form (`Vec<u8>`).
    /// - Parses the decrypted extension list (semicolon-separated) into `Vec<String>`.
    ///
    /// If the configuration file is missing or malformed, this will return
    /// a default (empty) configuration instead of failing hard.
    ///
    /// # Returns
    /// - `Ok(AppConfig)` with parsed and decoded settings.
    /// - `Err(anyhow::Error)` if any Base64 decoding operation fails.
    pub fn new() -> Result<Self, anyhow::Error> {
        match ConfigFile::new() {
            Ok(config) => {

                let mut app_config = AppConfig::default();
                if !config.resource_encode_key.is_empty() {
                    app_config.resource_encryption_key = Some(general_purpose::STANDARD.decode(config.resource_encode_key)?);
                }
                if !config.dec_table.is_empty() {
                    app_config.dec_table = Some(general_purpose::STANDARD.decode(config.dec_table)?);
                }
                if !config.ref_table.is_empty() {
                    app_config.ref_table = Some(general_purpose::STANDARD.decode(config.ref_table)?);
                }
                if !config.decrypted_extensions.is_empty() {
                    app_config.decrypted_extensions = Some(config.decrypted_extensions.split(';').map(str::to_string).collect());
                }
                Ok(app_config)
            },
            Err(_) => Ok(AppConfig::default())
        }
    }
}