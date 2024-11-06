// use emk_rs::types::{string_kv_pair, DataTypeOut};

pub fn main() {
    tracing_subscriber::fmt::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();
    println!("emk-rs: read");

    let data = include_bytes!("../examples/000001.emk");

    let emk = emk_rs::types::EmkFile::from_bytes(data).unwrap();

    println!("{:#?}", emk);

    // let mut reader = emk_rs::types::EmkReader::decrypt_default_key(&data.to_vec()).unwrap();

    // // let a = reader.read_header();
    // let b = reader.read_tags();

    // println!("{:#?}", b);

    // // let s = String::from_utf8_lossy(&a);
    // let data = reader.read_tag_data("SONG_INFO").unwrap();
    // let s = String::from_utf8_lossy(&data);

    // println!("{}", s);

    // let kv = string_kv_pair(s.to_string());
    // println!("{:#?}", kv);
}
