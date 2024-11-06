pub fn main() {
    println!("emk-rs: crack");

    let data = include_bytes!("../examples/000001.emk");

    let _ = emk_rs::util::xor_cracker(data);
}
