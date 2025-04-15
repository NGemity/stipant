use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{Cursor, Read, Seek, Write},
    path::Path,
    sync::Arc,
    thread,
};

use binrw::{BinRead, binrw};
use bytes::Buf;

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

/// Structure for usage in stripent
#[derive(Default, Debug, serde::Serialize)]
pub struct RZFile {
    pub base: IndexFile,
    pub hash: String,
    pub name: String,
    pub file: u8,
    pub found: bool, // For visualising
}

#[binrw]
#[brw(little)]
#[derive(Default, Debug, serde::Serialize)]
/// Structure of the data.000 file
pub struct IndexFile {
    pub str_len: u8,
    #[br(count = str_len)]
    pub hash: Vec<u8>,
    pub offset: u32,
    pub size: u32,
}

/// Collection of RZFiles
pub struct DataHandler {
    pub data_dir: String,
    pub export_dir: Option<String>,
    file_list: HashMap<String, Arc<RZFile>>,
    loop_list: Arc<HashMap<u8, Vec<Arc<RZFile>>>>,
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
            let _ = init_loop_list.insert(n as u8, Vec::<Arc<RZFile>>::new());
        }
        // Read file into buffer
        let mut buf = Vec::new();
        File::open(data_file)?.read_to_end(&mut buf)?;
        // decipher file
        RZFileManagement::cipher(&mut buf, &config.resource_encryption_key);

        let mut file_list = HashMap::new();
        let mut reader = Cursor::new(buf);

        while reader.has_remaining() {
            let data = IndexFile::read(&mut reader)?;

            let name = RZFileManagement::decode_filename(
                data.hash.clone(),
                &config.ref_table,
                &config.dec_table,
            )?;
            let hash = String::from_utf8(data.hash.clone())?;

            let rz_file = Arc::new(RZFile {
                hash: hash.clone(),
                name: name.clone(),
                file: RZFileManagement::get_file_no(hash.as_str()),
                base: data,
                found: true,
            });
            if let Some(val) = init_loop_list.get_mut(&rz_file.file) {
                val.push(rz_file.clone());
            };
            let _ = file_list.insert(name, rz_file);
        }

        for n in 1..9 {
            if let Some(val) = init_loop_list.get_mut(&n) {
                val.sort_by(|a, b| a.base.offset.partial_cmp(&b.base.offset).unwrap());
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

    pub fn get_entry_by_name(&self, file_name: &str) -> Option<Arc<RZFile>> {
        self.file_list.get(file_name).cloned()
    }

    pub fn len(&self) -> usize {
        self.file_list.len()
    }

    fn check_export_dir(&self) -> bool {
        if self.export_dir.is_none() {
            return false;
        }
        Path::new(&self.export_dir.as_ref().unwrap()).is_dir()
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
                    let path = Path::new(&data_dir).join(format!("data.00{}", i));
                    if !path.is_file() {
                        return;
                    }
                    // Open file
                    let mut file = OpenOptions::new().read(true).open(path).unwrap();
                    for rz_file in loop_list[&i].iter() {
                        let mut buf = vec![0u8; rz_file.base.size as usize];
                        file.seek(std::io::SeekFrom::Start(rz_file.base.offset.into()))
                            .unwrap();
                        file.read_exact(&mut buf).unwrap();

                        DataHandler::save_file(
                            &export_dir,
                            &rz_file.name,
                            rz_file,
                            &mut buf,
                            &cfg.resource_encryption_key,
                            &cfg.decrypted_extensions
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

            let mut buf = vec![0u8; rz_file.base.size as usize];
            // use scope for reading
            let mut file = File::open(path)?;
            file.seek(std::io::SeekFrom::Start(rz_file.base.offset.into()))?;
            file.read_exact(&mut buf)?;

            DataHandler::save_file(
                self.export_dir.as_ref().unwrap(),
                file_name,
                &rz_file,
                &mut buf,
                &self.config.resource_encryption_key,
                &self.config.decrypted_extensions,
            )?
        }
        Ok(())
    }

    fn save_file(
        export_dir: &str,
        file_name: &str,
        rz_file: &RZFile,
        buf: &mut [u8],
        resource_encode_key: &[u8],
        extensions: &[String],
    ) -> Result<(), anyhow::Error> {
        if DataHandler::is_encrypted(file_name, extensions) {
            RZFileManagement::cipher(buf, resource_encode_key);
        }

        let ext = Path::new(rz_file.name.as_str()).extension();
        if ext.is_none() {
            return Err(StipantError::NotAnExtension.into());
        }

        let mut new_file = Path::new(export_dir).join(ext.unwrap());
        if !new_file.is_dir() && fs::create_dir(&new_file).is_err() {
            println!(
                "Race condition during subdir creation: {}",
                new_file.display()
            );
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

    fn is_encrypted(file_name: &str, extensions: &[String]) -> bool {
        for n in extensions.iter() {
            if file_name.ends_with(n) {
                return false;
            }
        }
        true
    }
}

pub(crate) struct RZFileManagement {}

impl RZFileManagement {
    pub fn get_file_no(hash: &str) -> u8 {
        let lower = hash.to_lowercase();
        let lower_bytes = lower.as_bytes();
        let mut checksum = 0i32;
        for n in lower_bytes.iter() {
            checksum = checksum.wrapping_mul(31).wrapping_add(*n as i32);
        }

        if checksum < 0 {
            checksum *= -1
        }

        ((checksum & 0x7) + 1) as u8
    }

    pub fn decode_filename(
        hash: Vec<u8>,
        ref_table: &[u8],
        dec_table: &[u8],
    ) -> Result<String, anyhow::Error> {
        let mut name = hash[1..hash.len() - 1].to_vec();

        RZFileManagement::swap_string(&mut name);

        let mut depth: i32 = *ref_table.get(hash[hash.len() - 1] as usize).unwrap_or(&0) as i32;

        for n in name.iter_mut() {
            let mut compute_var = *n;
            for _ in 0..depth {
                compute_var = *dec_table.get(compute_var as usize).unwrap_or(&0);
            }

            *n = compute_var;

            // the following is identical, just safe for overflows
            // depth = (depth + (17 * compute_var as i32)) % 32 + 1;
            depth = (depth
                .checked_add((compute_var as i32).checked_mul(17).unwrap_or(0))
                .unwrap_or(0))
            .checked_rem(32)
            .unwrap_or(0)
            .checked_add(1)
            .unwrap_or(0);
        }

        Ok(String::from_utf8(name)?)
    }

    fn cipher(buffer: &mut [u8], resource_encode_key: &[u8]) -> u8 {
        let mut index = 0u8;
        for n in buffer.iter_mut() {
            *n ^= *resource_encode_key.get(index as usize).unwrap_or(&0);
            index = index.checked_add(1).unwrap_or(0);
        }
        index
    }

    fn swap_string(hash: &mut [u8]) {
        let median_pt13 = (0.33f32 * hash.len() as f32).floor() as usize;
        let median_pt23 = (0.66f32 * hash.len() as f32).floor() as usize;

        let (val1, val2) = (hash[median_pt23], hash[median_pt13]);

        (hash[median_pt23], hash[median_pt13]) = (hash[0], hash[1]);

        (hash[0], hash[1]) = (val1, val2);
    }
}
