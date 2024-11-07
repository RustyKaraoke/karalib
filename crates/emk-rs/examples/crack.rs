pub fn main() {
    tracing_subscriber::fmt::fmt()
        .with_max_level(tracing::Level::TRACE)
        .init();
    println!("emk-rs: crack");

    let data = include_bytes!("../examples/000001.emk");

    let emk1 = emk_rs::types::EmkFile::from_bytes(data).unwrap();

    let key = emk_rs::util::xor_cracker_alula(data).unwrap();

    println!("{:X?}", key);

    let emk = emk_rs::types::EmkFile::from_bytes_with_key(data, &key).unwrap();

    // println!("{:#?}", emk);
}
