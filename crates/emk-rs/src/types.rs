use flate2::read::ZlibDecoder;
use md5::{Digest, Md5};
use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
const MAGIC: [u8; 4] = [0x53, 0x46, 0x44, 0x53];

use tracing::debug;
type BoxedVec = Box<Vec<u8>>;

pub fn string_kv_pair(data: String) -> Vec<(String, String)> {
    data.lines()
        .filter(|s| !s.is_empty())
        .map(|s| {
            let mut split = s.split('=');
            let key = split.next().unwrap().to_string();
            let value = split.next().unwrap().to_string();
            (key, value)
        })
        .collect()
}
#[derive(Debug)]
pub struct EmkFile(pub Vec<Data>);

impl EmkFile {
    pub fn from_reader(reader: EmkReader) -> Result<Self, String> {
        let data = reader.into_emk_file()?;
        Ok(data)
    }

    pub fn read_from_path(path: &Path) -> Result<Self, String> {
        let data = std::fs::read(path).map_err(|e| e.to_string())?;
        let reader = EmkReader::decrypt_default_key(&data)?;
        Self::from_reader(reader)
    }

    pub fn from_bytes(data: &[u8]) -> Result<Self, String> {
        let reader = EmkReader::decrypt_default_key(data)?;
        Self::from_reader(reader)
    }

    pub fn from_bytes_with_key(data: &[u8], key: &[u8]) -> Result<Self, String> {
        let reader = EmkReader::decrypt(data, key)?;
        Self::from_reader(reader)
    }

    pub fn get_data(&self, tag: &str) -> Option<&Data> {
        self.0.iter().find(|data| data.tag == tag)
    }
}

// #[derive(Debug)]
pub struct Data {
    /// ID of the tag
    pub tag: String,

    /// Beginning offset of compressed data
    pub data_begin: u64,
    /// End offset of compressed data
    pub data_end: u64,
    /// MD5 hash of the compressed data
    pub md5_hash: [u8; 16],
    /// Uncompressed size of the data
    pub uncompressed_size: u64,

    // unknown fields
    pub unk2: bool,
    pub unk5: bool,
    pub unk6: bool,
    pub unk7: String,
    pub unk8: bool,

    pub data: TagData,
}

impl std::fmt::Debug for Data {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Data")
            .field("tag", &self.tag)
            .field("data_begin", &self.data_begin)
            .field("data_end", &self.data_end)
            .field("md5_hash", &hex::encode(self.md5_hash))
            .field("uncompressed_size", &self.uncompressed_size)
            .field("unk2", &self.unk2)
            .field("unk5", &self.unk5)
            .field("unk6", &self.unk6)
            .field("unk7", &self.unk7)
            .field("unk8", &self.unk8)
            .field(
                "data",
                match &self.data {
                    TagData::Header(h) => h,
                    TagData::Midi(_) => &"<MIDI>",
                    TagData::Lyrics(l) => {
                        let lyrics = String::from_utf8_lossy(l);
                        {
                            {
                                let formatted_lyrics = format!("{:?}", lyrics);
                                &*Box::leak(Box::new(formatted_lyrics))
                            }
                        }
                    }
                    TagData::Cursor(_) => &"<Cursor>",
                    TagData::SongInfo(s) => s,
                    TagData::Unknown(_) => &"<Unknown>",
                },
            )
            .finish()
    }
}

#[derive(Debug)]
pub enum TagData {
    Header(Header),
    Midi(BoxedVec),
    Lyrics(BoxedVec),
    Cursor(BoxedVec),
    SongInfo(Box<SongInfo>),
    Unknown(BoxedVec),
}

impl TagData {
    fn from_buf_with_tag(tag: &str, data: Vec<u8>) -> Self {
        match tag {
            "HEADER" => {
                let header_str = String::from_utf8_lossy(&data);
                TagData::Header(Header::from_kv(&header_str.to_string()))
            }
            "SONG_INFO" => {
                let song_info_str = String::from_utf8_lossy(&data);
                TagData::SongInfo(Box::new(SongInfo::from_kv(&song_info_str.to_string())))
            }
            "MIDI_DATA" => TagData::Midi(Box::new(data)),
            "LYRIC_DATA" => TagData::Lyrics(Box::new(data)),
            "CURSOR_DATA" => TagData::Cursor(Box::new(data)),
            _ => TagData::Unknown(Box::new(data)),
        }
    }
    pub fn from_reader(reader: &mut EmkReader) -> Result<Vec<Self>, String> {
        reader
            .read_tags()
            .iter()
            .map(|tag| {
                let tag_name = tag.get("tag").unwrap().to_string();

                let raw_data = reader
                    .read_tag_data(&tag_name)
                    .ok_or("Failed to read tag data")?;
                Ok(TagData::from_buf_with_tag(&tag_name, raw_data))
            })
            .collect::<Result<Vec<_>, _>>()
    }
}
#[derive(Debug)]
pub struct Header {
    pub signature: String,
    pub version: String,
}

impl Header {
    pub fn from_kv(data: &String) -> Self {
        let kv = string_kv_pair(data.to_string())
            .into_iter()
            .collect::<std::collections::HashMap<String, String>>();
        Self {
            signature: kv.get("SIGNATURE").unwrap().to_string(),
            version: kv.get("VERSION").unwrap().to_string(),
        }
    }
}
#[derive(Debug)]
pub struct SongInfo {
    /// ID of the EMK file
    pub code: String,
    /// Type of EMK file
    // TODO: make this an enum
    pub song_type: String,
    /// Subtitle type
    // TODO: make this an enum
    pub subtitle_type: String,
    /// Song title
    pub title: String,
    /// Key of the song
    pub key: String,
    /// Artist of the song
    pub artist: String,
    /// Language
    pub language: String,
    /// MIDI channel with the vocals
    pub vocal_channel: u8,
    /// Original file name
    pub file_name: String,
    /// Lyric title
    pub lyric_title: String,
    /// Start time of the song
    pub start_time: u32,
    /// End time of the song
    pub stop_time: u32,
    /// Tempo of the song
    pub tempo: u32,
}

impl SongInfo {
    pub fn from_kv(data: &String) -> Self {
        let kv = string_kv_pair(data.to_string())
            .into_iter()
            .collect::<std::collections::HashMap<String, String>>();
        Self {
            code: kv.get("CODE").unwrap().to_string(),
            song_type: kv.get("TYPE").unwrap().to_string(),
            subtitle_type: kv.get("SUB_TYPE").unwrap().to_string(),
            title: kv.get("TITLE").unwrap().to_string(),
            key: kv.get("KEY").unwrap().to_string(),
            artist: kv.get("ARTIST").unwrap().to_string(),
            language: kv.get("LANGUAGE").unwrap().to_string(),
            vocal_channel: kv.get("VOCAL_CHANNEL").unwrap().parse::<u8>().unwrap(),
            file_name: kv.get("FILE_NAME").unwrap().to_string(),
            lyric_title: kv.get("LYRIC_TITLE").unwrap().to_string(),
            start_time: kv.get("START_TIME").unwrap().parse::<u32>().unwrap(),
            stop_time: kv.get("STOP_TIME").unwrap().parse::<u32>().unwrap(),
            tempo: kv.get("TEMPO").unwrap().parse::<u32>().unwrap(),
        }
    }
}

#[derive(Debug, Clone, Copy, FromPrimitive)]
pub enum DataType {
    Byte = 2,
    Short = 3,
    Int = 4,
    String = 6,
}

#[derive(Clone)]
pub enum DataTypeOut {
    Byte(u8),
    Short(u16),
    Int(u32),
    String(String),
    Data(Vec<u8>),
}

use std::{fmt, io::Read, path::Path};

use crate::util::{xor, xor_cracker_alula, xor_cracker_bruteforce, EMK_MAGIC};

impl fmt::Display for DataTypeOut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataTypeOut::Byte(b) => write!(f, "{}", b),
            DataTypeOut::Short(s) => write!(f, "{}", s),
            DataTypeOut::Int(i) => write!(f, "{}", i),
            DataTypeOut::String(s) => write!(f, "{}", s),
            DataTypeOut::Data(d) => write!(f, "{:X?}", d),
        }
    }
}

impl fmt::Debug for DataTypeOut {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataTypeOut::Byte(b) => write!(f, "Byte({})", b),
            DataTypeOut::Short(s) => write!(f, "Short({})", s),
            DataTypeOut::Int(i) => write!(f, "Int({})", i),
            DataTypeOut::String(s) => write!(f, "String({})", s),
            DataTypeOut::Data(d) => write!(f, "Data({:X?})", d),
        }
    }
}

impl From<DataTypeOut> for Vec<u8> {
    fn from(val: DataTypeOut) -> Self {
        match val {
            DataTypeOut::Byte(b) => vec![b],
            DataTypeOut::Short(s) => s.to_le_bytes().to_vec(),
            DataTypeOut::Int(i) => i.to_le_bytes().to_vec(),
            DataTypeOut::String(s) => s.as_bytes().to_vec(),
            DataTypeOut::Data(d) => d,
        }
    }
}

impl From<DataTypeOut> for u8 {
    fn from(val: DataTypeOut) -> Self {
        match val {
            DataTypeOut::Byte(b) => b,
            DataTypeOut::Short(s) => s as u8,
            DataTypeOut::Int(i) => i as u8,
            DataTypeOut::String(s) => s.parse::<u8>().unwrap(),
            DataTypeOut::Data(d) => d[0],
        }
    }
}

impl From<DataTypeOut> for u16 {
    fn from(val: DataTypeOut) -> Self {
        match val {
            DataTypeOut::Byte(b) => b as u16,
            DataTypeOut::Short(s) => s,
            DataTypeOut::Int(i) => i as u16,
            DataTypeOut::String(s) => s.parse::<u16>().unwrap(),
            DataTypeOut::Data(d) => u16::from_le_bytes(d[0..2].try_into().unwrap()),
        }
    }
}

impl From<DataTypeOut> for u32 {
    fn from(val: DataTypeOut) -> Self {
        match val {
            DataTypeOut::Byte(b) => b as u32,
            DataTypeOut::Short(s) => s as u32,
            DataTypeOut::Int(i) => i,
            DataTypeOut::String(s) => s.parse::<u32>().unwrap(),
            DataTypeOut::Data(d) => u32::from_le_bytes(d[0..4].try_into().unwrap()),
        }
    }
}

impl From<DataTypeOut> for u64 {
    fn from(val: DataTypeOut) -> Self {
        match val {
            DataTypeOut::Byte(b) => b as u64,
            DataTypeOut::Short(s) => s as u64,
            DataTypeOut::Int(i) => i as u64,
            DataTypeOut::String(s) => s.parse::<u64>().unwrap(),
            DataTypeOut::Data(d) => u64::from_le_bytes(d[0..8].try_into().unwrap()),
        }
    }
}

pub struct EmkReader {
    data: Vec<u8>,
    header: Vec<u8>,
    pos: usize,
}

impl EmkReader {
    pub fn decrypt(data: &[u8], key: &[u8]) -> Result<Self, String> {
        let data = xor(data, key)?;

        let header_pos = u64::from_le_bytes(data[0x22..0x2a].try_into().unwrap()) as usize;
        let header_end = u64::from_le_bytes(data[0x2a..0x32].try_into().unwrap()) as usize;

        let header = data
            .get(header_pos..header_end)
            .ok_or("Invalid header range")?
            .to_vec();

        Ok(Self {
            data: data.clone(),
            header,
            pos: 0,
        })
    }

    pub fn decrypt_default_key(data: &[u8]) -> Result<Self, String> {
        Self::decrypt(data, EMK_MAGIC.to_be_bytes().as_ref())
    }

    // /// Attempt to crack the key using Alula's algorithm
    // pub fn try_decrypt(data: &[u8]) -> Result<(Self, Vec<u8>), String> {
    //     // Try every possible u64
    //     // use rayon::prelude::*;

    //     let key = xor_cracker_alula(data).map_err(|e| e.to_string());
    //     if let Ok(key) = key {
    //         return Ok((Self::decrypt(data, &key)?, key));
    //     }

    //     Err("Failed to decrypt".to_string())
    // }

    fn check_magic(&mut self, magic: &[u8]) -> bool {
        let data = self.header[self.pos..self.pos + magic.len()].to_vec();
        if data != magic {
            return false;
        }
        // Oh yeah, we need to skip magic bytes
        self.pos += magic.len();
        true
    }

    #[allow(dead_code)]
    fn skip(&mut self, n: usize) {
        self.pos += n;
    }

    fn read_byte(&mut self) -> u8 {
        let byte = self.header[self.pos];
        self.pos += 1;

        byte
    }

    fn read_u16(&mut self) -> u16 {
        let res = u16::from_le_bytes(self.header[self.pos..self.pos + 2].try_into().unwrap());
        self.pos += 2;
        res
    }

    fn read_u32(&mut self) -> u32 {
        let res = u32::from_le_bytes(self.header[self.pos..self.pos + 4].try_into().unwrap());
        self.pos += 4;
        res
    }
    fn read_string(&mut self) -> String {
        let len = self.read_byte() as usize;
        let str = String::from_utf8(self.header[self.pos..self.pos + len].to_vec()).unwrap();
        self.pos += len;
        str
    }

    fn read_tag(&mut self) -> DataTypeOut {
        let byte = self.read_byte();

        // debug!("Reading tag: {}", byte);

        let tag: Option<DataType> = FromPrimitive::from_u8(byte);
        match tag {
            Some(DataType::Byte) => {
                let v = self.read_byte();
                // debug!("Byte: {}", v);
                DataTypeOut::Byte(v)
            }
            Some(DataType::Short) => {
                let v = self.read_u16();
                // debug!("Short: {}", v);
                DataTypeOut::Short(v)
            }
            Some(DataType::Int) => {
                let v = self.read_u32();
                // debug!("Int: {}", v);
                DataTypeOut::Int(v)
            }
            Some(DataType::String) => {
                let v = self.read_string();
                // debug!("String: {}", v);
                DataTypeOut::String(v)
            }

            None => todo!(),
        }
    }

    pub fn read_header(&mut self) -> Result<(), String> {
        // self.decrypt();
        while self.pos < self.header.len() {
            if !self.check_magic(MAGIC.as_ref()) {
                return Err("Magic check failed".to_string());
            }
            let tag = self.read_tag();
            let uncompressed_size = self.read_tag();
            let _unk2 = self.read_tag();
            let data_begin = std::convert::Into::<u16>::into(self.read_tag());
            let data_end = std::convert::Into::<u16>::into(self.read_tag());
            let _unk5 = self.read_tag();
            let _unk6 = self.read_tag();
            let md5_hash = {
                let res = self.data[self.pos..self.pos + 16].to_vec();
                self.pos += 16;
                res
            };
            let _unk7 = self.read_tag();
            let _unk8 = self.read_tag();

            debug!(
                "=== Header ===\nTag: {:?}\nUncompressed size: {:?}\nUnk2: {:?}\nData begin: {:?}\nData end: {:?}\nUnk5: {:?}\nUnk6: {:?}\nMD5 hash: {:?}\nUnk7: {:?}\nUnk8: {:?}",
                tag, uncompressed_size, _unk2, data_begin, data_end, _unk5, _unk6, md5_hash, _unk7, _unk8
            );

            // compressed data
            let mut hasher = Md5::new();
            let compressed_data = self.data[data_begin as usize..data_end as usize].to_vec();

            // check md5 hash

            // ? md5 hash is wrong for some reason
            let raw_data = {
                let mut buf = Vec::new();
                let mut decoder = ZlibDecoder::new(compressed_data.as_slice());
                decoder.read_to_end(&mut buf).map_err(|e| e.to_string())?;
                buf
            };
            hasher.update(raw_data.as_slice());
            let hash = hasher.finalize_reset();
            debug!("Hash: {:?}", hash);
            debug!("Embedded Hash: {:?}", md5_hash);

            if let DataTypeOut::String(s) = tag {
                if let "HEADER" = s.as_str() {
                    debug!("--- HEADER ---");
                    debug!(
                        "{}",
                        String::from_utf8(raw_data).map_err(|e| e.to_string())?
                    );
                    debug!("--- END HEADER ---");
                }
            }
        }

        // reset pos to 0
        self.pos = 0;
        Ok(())
    }
    pub fn read_tags(&mut self) -> Vec<std::collections::BTreeMap<String, DataTypeOut>> {
        let mut tags = Vec::new();
        while self.pos < self.header.len() {
            if !self.check_magic(MAGIC.as_ref()) {
                break;
            }
            let mut tag_map = std::collections::BTreeMap::new();
            let tag = self.read_tag();
            let uncompressed_size = self.read_tag();
            let unk2 = self.read_tag();
            let data_begin = self.read_tag();
            let data_end = self.read_tag();
            let unk5 = self.read_tag();
            let unk6 = self.read_tag();
            let md5_hash = {
                let res = self.data[self.pos..self.pos + 16].to_vec();
                self.pos += 16;
                res
            };
            let unk7 = self.read_tag();
            let unk8 = self.read_tag();

            tag_map.insert("tag".to_string(), tag.clone());
            tag_map.insert("uncompressed_size".to_string(), uncompressed_size);
            tag_map.insert("unk2".to_string(), unk2);
            tag_map.insert("data_begin".to_string(), data_begin.clone());
            tag_map.insert("data_end".to_string(), data_end.clone());
            tag_map.insert("unk5".to_string(), unk5);
            tag_map.insert("unk6".to_string(), unk6);
            tag_map.insert(
                "md5_hash".to_string(),
                DataTypeOut::String(hex::encode(md5_hash)),
            );
            tag_map.insert("unk7".to_string(), unk7);
            tag_map.insert("unk8".to_string(), unk8);

            // Read and decompress the inner data
            // let data_begin: u32 = data_begin.into();
            // let data_end: u32 = data_end.into();
            // let compressed_data = self.data[data_begin as usize..data_end as usize].to_vec();
            // let raw_data = {
            //     let mut buf = Vec::new();
            //     let mut decoder = ZlibDecoder::new(compressed_data.as_slice());
            //     decoder.read_to_end(&mut buf).unwrap();
            //     buf
            // };

            // tag_map.insert("raw_data".to_string(), TagOut::Data(raw_data.clone()));

            tags.push(tag_map);
        }

        // reset pos to 0
        self.pos = 0;
        tags
    }

    pub fn into_emk_file(mut self) -> Result<EmkFile, String> {
        let mut data = Vec::new();
        while self.pos < self.header.len() {
            if !self.check_magic(MAGIC.as_ref()) {
                return Err("Magic check failed".to_string());
            }
            let tag = self.read_tag();
            let tag_string = if let DataTypeOut::String(s) = tag {
                s
            } else {
                return Err("Invalid tag type".to_string());
            };
            let uncompressed_size = self.read_tag();
            let unk2 = self.read_tag();
            let data_begin = self.read_tag();
            let data_end = self.read_tag();
            let unk5 = self.read_tag();
            let unk6 = self.read_tag();
            let md5_hash = {
                let res = self.data[self.pos..self.pos + 16].to_vec();
                self.pos += 16;
                res
            };
            let unk7 = self.read_tag();
            let unk8 = self.read_tag();

            let data_begin: u32 = data_begin.clone().into();
            let data_end: u32 = data_end.clone().into();
            let compressed_data = self.data[data_begin as usize..data_end as usize].to_vec();
            let raw_data = {
                let mut buf = Vec::new();
                let mut decoder = ZlibDecoder::new(compressed_data.as_slice());
                decoder.read_to_end(&mut buf).map_err(|e| e.to_string())?;
                buf
            };

            data.push(Data {
                tag: tag_string.clone(),
                data_begin: data_begin.into(),
                data_end: data_end.into(),
                md5_hash: md5_hash
                    .try_into()
                    .map_err(|_| "Invalid MD5 hash length".to_string())?,
                uncompressed_size: uncompressed_size.into(),
                unk2: if let DataTypeOut::Byte(b) = unk2 {
                    b != 0
                } else {
                    return Err("Invalid unk2 type".to_string());
                },
                unk5: if let DataTypeOut::Byte(b) = unk5 {
                    b != 0
                } else {
                    return Err("Invalid unk5 type".to_string());
                },
                unk6: if let DataTypeOut::Byte(b) = unk6 {
                    b != 0
                } else {
                    return Err("Invalid unk6 type".to_string());
                },
                unk7: if let DataTypeOut::String(s) = unk7 {
                    s
                } else {
                    return Err("Invalid unk7 type".to_string());
                },
                unk8: if let DataTypeOut::Byte(b) = unk8 {
                    b != 0
                } else {
                    return Err("Invalid unk8 type".to_string());
                },
                data: TagData::from_buf_with_tag(&tag_string, raw_data),
            });
        }

        Ok(EmkFile(data))
    }

    pub fn read_tag_data(&mut self, tag: &str) -> Option<Vec<u8>> {
        let binding = self.read_tags();
        let t = tag.to_string();
        let tag = binding
            .iter()
            .find(|tag| matches!(tag.get("tag"), Some(DataTypeOut::String(s)) if *s == *t));

        if let Some(tag) = tag {
            // get data_begin and data_end

            let data_begin = tag.get("data_begin")?;
            let data_end = tag.get("data_end")?;

            let data_begin: u32 = data_begin.clone().into();
            let data_end: u32 = data_end.clone().into();
            let compressed_data = self.data[data_begin as usize..data_end as usize].to_vec();
            let raw_data = {
                let mut buf = Vec::new();
                let mut decoder = ZlibDecoder::new(compressed_data.as_slice());
                decoder.read_to_end(&mut buf).unwrap();
                buf
            };

            return Some(raw_data);
        }

        None
    }
}
