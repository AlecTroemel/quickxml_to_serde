extern crate quickxml_to_serde;
use quickxml_to_serde::{xml_string_to_json, Config, NullValue};

fn main() {
    let xml = r#"<?xml version="1.0" encoding="utf-8"?><a attr1="1"><b><c attr2="001">some text</c></b></a>"#;
    let conf = Config::new_with_defaults();
    let json = xml_string_to_json(xml.to_owned(), &conf);
    println!("{}", json.expect("Malformed XML").to_string());

    let conf = Config::new_with_custom_values(true, "", "txt", NullValue::Null);
    let json = xml_string_to_json(xml.to_owned(), &conf);
    println!("{}", json.expect("Malformed XML").to_string());
}
