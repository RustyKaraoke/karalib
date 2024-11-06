pub mod types;
pub mod util;

#[test]
#[tracing_test::traced_test]
fn read_emk() {
    let data = include_bytes!("../examples/000001.emk");

    // tracing_subscriber::fmt::
    // let mut file = std::fs::File::open("000001.emk").unwrap();
    // let mut data = Vec::new();
    // file.read_to_end(&mut data).unwrap();
    use types::EmkFile;
    // header_pos is 0x22
    // let header: Vec<u8> = data[0x22..0x2a].to_vec();

    {
        let file = EmkFile::from_bytes(data).unwrap();

        // file.get_data("SONG_INFO").unwrap();

        let data = &file.get_data("SONG_INFO").unwrap().data;
        if let types::TagData::SongInfo(s) = data {
            // println!("{:#?}", s);
            assert_eq!(s.code, "000001");
        }
        // assert_eq!()

        // let (mut reader, mut _key) = EmkReader::try_decrypt_brute_force(data.to_vec()).unwrap();
        // let mut reader = EmkReader::new(data.to_vec()).unwrap();

        // let _ = reader.read_header();

        // util::test_xor_crack(data.to_vec());
    }
}
