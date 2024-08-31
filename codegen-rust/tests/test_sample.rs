#[test]
fn parse() {
    std::env::set_var("OUT_DIR", ".");
    let mut generated = Vec::new();
    assert!(codegen_rust::Builder::new("tests/sample.toml")
        .compile_to_writer(&mut generated)
        .is_ok());

    let generated = String::from_utf8(generated).expect("generated non-utf8 data");
    let expected = "pub struct MyMessage {
    pub a: String,
    pub b: u32,
}
#[repr(u32)]
pub enum MyEnum {
    OptionA = 1u32,
    OptionB = 2u32,
}
pub trait MyService {
    async fn my_call(&self, input: MyMessage) -> MyEnum;
}
";
    assert_eq!(expected, generated);
}
