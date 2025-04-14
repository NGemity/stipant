use std::{
    collections::HashMap,
    fs::{self, File, OpenOptions},
    io::{Cursor, Read, Seek, Write},
    path::Path,
    sync::Arc, thread,
};


use binrw::{BinRead, binrw};
use bytes::Buf;

use thiserror::Error;

const RESOURCE_ENCODE_KEY: [u8; 256] = [
    0x77, 0xe8, 0x5e, 0xec, 0xb7, 0x4e, 0xc1, 0x87, 0x4f, 0xe6, 0xf5, 0x3c, 0x1f, 0xb3, 0x15, 0x43,
    0x6a, 0x49, 0x30, 0xa6, 0xbf, 0x53, 0xa8, 0x35, 0x5b, 0xe5, 0x9e, 0x0e, 0x41, 0xec, 0x22, 0xb8,
    0xd4, 0x80, 0xa4, 0x8c, 0xce, 0x65, 0x13, 0x1d, 0x4b, 0x08, 0x5a, 0x6a, 0xbb, 0x6f, 0xad, 0x25,
    0xb8, 0xdd, 0xcc, 0x77, 0x30, 0x74, 0xac, 0x8c, 0x5a, 0x4a, 0x9a, 0x9b, 0x36, 0xbc, 0x53, 0x0a,
    0x3c, 0xf8, 0x96, 0x0b, 0x5d, 0xaa, 0x28, 0xa9, 0xb2, 0x82, 0x13, 0x6e, 0xf1, 0xc1, 0x93, 0xa9,
    0x9e, 0x5f, 0x20, 0xcf, 0xd4, 0xcc, 0x5b, 0x2e, 0x16, 0xf5, 0xc9, 0x4c, 0xb2, 0x1c, 0x57, 0xee,
    0x14, 0xed, 0xf9, 0x72, 0x97, 0x22, 0x1b, 0x4a, 0xa4, 0x2e, 0xb8, 0x96, 0xef, 0x4b, 0x3f, 0x8e,
    0xab, 0x60, 0x5d, 0x7f, 0x2c, 0xb8, 0xad, 0x43, 0xad, 0x76, 0x8f, 0x5f, 0x92, 0xe6, 0x4e, 0xa7,
    0xd4, 0x47, 0x19, 0x6b, 0x69, 0x34, 0xb5, 0x0e, 0x62, 0x6d, 0xa4, 0x52, 0xb9, 0xe3, 0xe0, 0x64,
    0x43, 0x3d, 0xe3, 0x70, 0xf5, 0x90, 0xb3, 0xa2, 0x06, 0x42, 0x02, 0x98, 0x29, 0x50, 0x3f, 0xfd,
    0x97, 0x58, 0x68, 0x01, 0x8c, 0x1e, 0x0f, 0xef, 0x8b, 0xb3, 0x41, 0x44, 0x96, 0x21, 0xa8, 0xda,
    0x5e, 0x8b, 0x4a, 0x53, 0x1b, 0xfd, 0xf5, 0x21, 0x3f, 0xf7, 0xba, 0x68, 0x47, 0xf9, 0x65, 0xdf,
    0x52, 0xce, 0xe0, 0xde, 0xec, 0xef, 0xcd, 0x77, 0xa2, 0x0e, 0xbc, 0x38, 0x2f, 0x64, 0x12, 0x8d,
    0xf0, 0x5c, 0xe0, 0x0b, 0x59, 0xd6, 0x2d, 0x99, 0xcd, 0xe7, 0x01, 0x15, 0xe0, 0x67, 0xf4, 0x32,
    0x35, 0xd4, 0x11, 0x21, 0xc3, 0xde, 0x98, 0x65, 0xed, 0x54, 0x9d, 0x1c, 0xb9, 0xb0, 0xaa, 0xa9,
    0x0c, 0x8a, 0xb4, 0x66, 0x60, 0xe1, 0xff, 0x2e, 0xc8, 0x00, 0x43, 0xa9, 0x67, 0x37, 0xdb, 0x9c,
];

const DEC_TABLE: [u8; 128] = [
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x21, 0x64, 0x00, 0x33, 0x37, 0x2d, 0x23, 0x62, 0x5a, 0x47, 0x00, 0x5f, 0x25, 0x36, 0x2c, 0x00,
    0x43, 0x35, 0x57, 0x70, 0x58, 0x7e, 0x4b, 0x2b, 0x30, 0x4c, 0x00, 0x79, 0x00, 0x52, 0x00, 0x00,
    0x44, 0x48, 0x68, 0x63, 0x61, 0x4d, 0x4e, 0x45, 0x6e, 0x66, 0x65, 0x40, 0x71, 0x59, 0x27, 0x29,
    0x34, 0x6f, 0x53, 0x46, 0x7d, 0x69, 0x38, 0x50, 0x28, 0x3b, 0x74, 0x39, 0x00, 0x32, 0x3d, 0x31,
    0x6a, 0x5e, 0x51, 0x7b, 0x67, 0x2e, 0x6c, 0x20, 0x56, 0x75, 0x42, 0x5b, 0x26, 0x5d, 0x72, 0x73,
    0x6d, 0x6b, 0x76, 0x77, 0x55, 0x78, 0x54, 0x24, 0x49, 0x4a, 0x7a, 0x4f, 0x00, 0x41, 0x60, 0x00,
];

const REF_TABLE: [u8; 128] = [
    0x54, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
    0x00, 0x4b, 0x00, 0x16, 0x1c, 0x49, 0x01, 0x1d, 0x2a, 0x42, 0x00, 0x11, 0x12, 0x27, 0x00, 0x00,
    0x20, 0x4d, 0x33, 0x4f, 0x43, 0x0a, 0x44, 0x39, 0x1b, 0x4a, 0x00, 0x3e, 0x00, 0x3b, 0x00, 0x00,
    0x46, 0x28, 0x25, 0x18, 0x31, 0x48, 0x23, 0x38, 0x2d, 0x14, 0x19, 0x3a, 0x1f, 0x52, 0x04, 0x0e,
    0x21, 0x3d, 0x40, 0x3f, 0x02, 0x15, 0x10, 0x37, 0x2c, 0x0f, 0x2e, 0x4e, 0x00, 0x22, 0x00, 0x03,
    0x0c, 0x35, 0x3c, 0x4c, 0x06, 0x24, 0x50, 0x36, 0x2f, 0x13, 0x47, 0x17, 0x45, 0x51, 0x26, 0x09,
    0x2b, 0x1a, 0x0d, 0x05, 0x41, 0x29, 0x0b, 0x30, 0x08, 0x32, 0x53, 0x07, 0x00, 0x34, 0x1e, 0x00,
];

const DECRYPTED_EXTENSIONS: [&str; 6] = ["dds", "cob", "naf", "nx3", "nfm", "tga"];

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
    pub found: bool // For visualising
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
    loop_list: HashMap<u8, Vec<Arc<RZFile>>>,
}

impl DataHandler {
    pub fn new(filepath: &str) -> Result<Self, anyhow::Error> {
        let path = Path::new(filepath);
        if !path.is_dir() {
            return Err(StipantError::NotADirectory.into());
        }

        let data_file = path.join("data.000");
        if !data_file.is_file() {
            return Err(StipantError::NoDataPath(0).into());
        }

        let mut loop_list = HashMap::new();
        for n in 1..9 {
            let _ = loop_list.insert(n as u8, Vec::<Arc<RZFile>>::new());
        }
        // Read file into buffer
        let mut buf = Vec::new();
        File::open(data_file)?.read_to_end(&mut buf)?;
        // decipher file
        RZFileManagement::cipher(&mut buf);

        let mut file_list = HashMap::new();
        let mut reader = Cursor::new(buf);

        while reader.has_remaining() {
            let data = IndexFile::read(&mut reader)?;

            let name = RZFileManagement::decode_filename(data.hash.clone())?;
            let hash = String::from_utf8(data.hash.clone())?;

            let rz_file = Arc::new(RZFile {
                hash: hash.clone(),
                name: name.clone(),
                file: RZFileManagement::get_file_no(hash.as_str()),
                base: data,
                found: true
            });
            if let Some(val) = loop_list.get_mut(&rz_file.file) {
                val.push(rz_file.clone());
            };
            let _ = file_list.insert(name, rz_file);
        }

        for n in 1..9 {
            if let Some(val) = loop_list.get_mut(&n) {
                val.sort_by(|a, b| a.base.offset.partial_cmp(&b.base.offset).unwrap());
            };
        }

        Ok(Self {
            file_list,
            loop_list,
            data_dir: filepath.to_string(),
            export_dir: None,
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
                        file.seek(std::io::SeekFrom::Start(rz_file.base.offset.into())).unwrap();
                        file.read_exact(&mut buf).unwrap();
            
                        DataHandler::save_file(&export_dir, &rz_file.name, rz_file, &mut buf).unwrap();
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
            
            DataHandler::save_file(self.export_dir.as_ref().unwrap(), file_name, &rz_file, &mut buf)?
        }
        Ok(())
    }

    fn save_file(export_dir: &str, file_name: &str, rz_file: &RZFile, buf: &mut [u8]) -> Result<(), anyhow::Error> {
        if DataHandler::is_encrypted(file_name) {
            RZFileManagement::cipher( buf);
        }

        let ext = Path::new(rz_file.name.as_str()).extension();
        if ext.is_none() {
            return Err(StipantError::NotAnExtension.into());
        }

        let mut new_file = Path::new(export_dir).join(ext.unwrap());
        if !new_file.is_dir() && fs::create_dir(&new_file).is_err() {
            println!("Race condition during subdir creation: {}", new_file.display());
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

    fn is_encrypted(file_name: &str) -> bool {
        for n in DECRYPTED_EXTENSIONS.iter() {
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

    pub fn decode_filename(hash: Vec<u8>) -> Result<String, anyhow::Error> {
        let mut name = hash[1..hash.len() - 1].to_vec();

        RZFileManagement::swap_string(&mut name);

        let mut depth: i32 = REF_TABLE[hash[hash.len() - 1] as usize] as i32;

        for n in name.iter_mut() {
            let mut compute_var = *n;
            for _ in 0..depth {
                compute_var = DEC_TABLE[compute_var as usize];
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

    fn cipher(buffer: &mut [u8]) -> u8 {
        let mut index = 0u8;
        for n in buffer.iter_mut() {
            *n ^= RESOURCE_ENCODE_KEY[index as usize];
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
