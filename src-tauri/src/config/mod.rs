use config::{Config, ConfigError, Environment, File};
use serde_derive::Deserialize;
use base64::{engine::general_purpose, Engine as _};

#[derive(Debug, Deserialize)]
struct ConfigFile {
    pub resource_encode_key: String,
    pub dec_table: String,
    pub ref_table: String,
    pub decrypted_extensions: String
}

impl Default for ConfigFile {
    fn default() -> Self {
        Self { 
            resource_encode_key: String::new(),
            dec_table: String::new(),
            ref_table: String::new(),
            decrypted_extensions: String::new()
        }
    }
}

impl ConfigFile {
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

pub struct AppConfig {
    pub resource_encryption_key: Option<Vec<u8>>,
    pub dec_table: Option<Vec<u8>>,
    pub ref_table: Option<Vec<u8>>,
    pub decrypted_extensions: Option<Vec<String>>
}

impl AppConfig {
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

impl Default for AppConfig {
    fn default() -> Self {
        Self { 
            resource_encryption_key: None, 
            dec_table: None,
            ref_table: None,
            decrypted_extensions: None
        }
    }
}
