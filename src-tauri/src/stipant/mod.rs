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

#[derive(Error, Debug)]
enum StipantError {
    #[error("Invalid file path, data.00{0} not found.")]
    NoDataPath(u8),
    #[error("Not a directory.")]
    NotADirectory,
    #[error("Not a file extension.")]
    NotAnExtension,
    #[error("Invalid export directory!")]
    InvalidExportDirectory,
}

#[derive(Default, serde::Serialize)]
pub struct RZData {
    //pub base: rzfile::file::IndexFile,
    pub hash: String,
    pub name: String,
    pub file: u8,
    pub found: bool, // For visualising
    pub size: u32,
    pub offset: u32,
}

/// Collection of RZFiles
pub struct DataHandler {
    pub data_dir: String,
    pub export_dir: Option<String>,
    file_list: HashMap<String, Arc<RZData>>,
    loop_list: Arc<HashMap<u8, Vec<Arc<RZData>>>>,
    config: Arc<AppConfig>,
}

impl DataHandler {
    pub fn new(filepath: &str, config: Arc<AppConfig>) -> Result<Self, anyhow::Error> {
        let path = Path::new(filepath);
        if !path.is_dir() {
            return Err(StipantError::NotADirectory.into());
        }

        let data_file = path.join("data.000");
        if !data_file.is_file() {
            return Err(StipantError::NoDataPath(0).into());
        }

        let mut init_loop_list = HashMap::new();
        for n in 1..9 {
            let _ = init_loop_list.insert(n as u8, Vec::<Arc<RZData>>::new());
        }
        // Read file into buffer
        let mut buf = Vec::new();
        File::open(data_file)?.read_to_end(&mut buf)?;

        // Parse index file
        let file = parse_index(&mut buf, config.resource_encryption_key.as_deref())?;
        let mut file_list = HashMap::new();

        for entry in file {
            let hash = String::from_utf8(entry.hash.clone())?;
            let rz_file = Arc::new(RZData {
                hash: hash.clone(),
                name: rzfile::name::decode_file_name(
                    &hash,
                    config.dec_table.as_deref(),
                    config.dec_table.as_deref(),
                    true,
                )?,
                file: rzfile::name::get_file_no(&hash),
                found: true,
                size: entry.size,
                offset: entry.offset,
            });

            if let Some(val) = init_loop_list.get_mut(&rz_file.file) {
                val.push(rz_file.clone());
            };
            let _ = file_list.insert(rz_file.name.clone(), rz_file);
        }

        for n in 1..9 {
            if let Some(val) = init_loop_list.get_mut(&n) {
                val.sort_by(|a, b| a.offset.cmp(&b.offset));
            };
        }

        Ok(Self {
            file_list,
            loop_list: Arc::new(init_loop_list),
            data_dir: filepath.to_string(),
            export_dir: None,
            config,
        })
    }

    pub fn set_export_dir(&mut self, export_dir: &str) {
        self.export_dir = Some(export_dir.to_string());
    }

    pub fn get_entry_by_name(&self, file_name: &str) -> Option<Arc<RZData>> {
        self.file_list.get(file_name).cloned()
    }

    pub fn len(&self) -> usize {
        self.file_list.len()
    }

    fn check_export_dir(&self) -> bool {
        self.export_dir
            .as_ref()
            .map(|p| Path::new(p).is_dir())
            .unwrap_or(false)
    }

    pub fn dump_all(&self) {
        if !self.check_export_dir() {
            return;
        }

        let threads: Vec<_> = (1..9)
            .map(|i| {
                let data_dir = self.data_dir.clone();
                let loop_list = self.loop_list.clone();
                let export_dir = self.export_dir.as_ref().unwrap().clone();
                let cfg = self.config.clone();
                thread::spawn(move || {
                    // Get path
                    let path = Path::new(&data_dir).join(format!("data.00{i}"));
                    if !path.is_file() {
                        return;
                    }
                    // Open file
                    let mut file = OpenOptions::new().read(true).open(path).unwrap();
                    for rz_file in loop_list[&i].iter() {
                        let mut buf = vec![0u8; rz_file.size as usize];
                        let _ = file
                            .seek(std::io::SeekFrom::Start(rz_file.offset.into()))
                            .and_then(|_| file.read_exact(&mut buf));

                        DataHandler::save_file(
                            &export_dir,
                            &rz_file.name,
                            rz_file,
                            &mut buf,
                            cfg.clone(),
                        )
                        .unwrap();
                    }
                })
            })
            .collect();

        for handle in threads {
            handle.join().unwrap();
        }
    }

    pub fn dump_by_filename(&self, file_name: &str) -> Result<(), anyhow::Error> {
        if !self.check_export_dir() {
            return Err(StipantError::InvalidExportDirectory.into());
        }

        if let Some(rz_file) = self.get_entry_by_name(file_name) {
            let path = Path::new(&self.data_dir).join(format!("data.00{}", rz_file.file));
            if !path.is_file() {
                return Err(StipantError::NoDataPath(rz_file.file).into());
            }

            let mut buf = vec![0u8; rz_file.size as usize];
            // use scope for reading
            let mut file = File::open(path)?;
            file.seek(std::io::SeekFrom::Start(rz_file.offset.into()))?;
            file.read_exact(&mut buf)?;

            DataHandler::save_file(
                self.export_dir.as_ref().unwrap(),
                file_name,
                &rz_file,
                &mut buf,
                self.config.clone(),
            )?
        }
        Ok(())
    }

    fn save_file(
        export_dir: &str,
        file_name: &str,
        rz_file: &RZData,
        buf: &mut [u8],
        config: Arc<AppConfig>,
    ) -> Result<(), anyhow::Error> {
        if rzfile::file::is_encrypted(file_name, config.decrypted_extensions.clone()) {
            rzfile::file::cipher(buf, config.resource_encryption_key.as_deref());
        }

        let ext = Path::new(&rz_file.name).extension()
            .ok_or(StipantError::NotAnExtension)?;

        let mut new_file = Path::new(export_dir).join(ext);

        if !new_file.is_dir() {
            if let Err(e) = fs::create_dir(&new_file) {
                // Ignore race condition, it's fine I guess
                if e.kind() != io::ErrorKind::AlreadyExists {
                    return Err(e)?;
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
}
