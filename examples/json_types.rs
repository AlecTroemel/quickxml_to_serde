extern crate quickxml_to_serde;
#[cfg(feature = "json_types")]
use quickxml_to_serde::{xml_string_to_json, Config, JsonArray, JsonType};

#[cfg(feature = "json_types")]
fn main() {
    let xml = r#"<a attr1="007"><b attr1="7">true</b></a>"#;

    // custom config values for 1 attribute and a text node
    let conf = Config::new_with_defaults()
        .add_json_type_override("/a/b/@attr1", JsonArray::Infer(JsonType::AlwaysString))
        .add_json_type_override("/a/b", JsonArray::Infer(JsonType::AlwaysString));
    let json = xml_string_to_json(String::from(xml), &conf);
    println!("{}", json.expect("Malformed XML").to_string());
}

#[cfg(not(feature = "json_types"))]
fn main() {
    println!("Run this example with `--features json_types` parameter");
}
