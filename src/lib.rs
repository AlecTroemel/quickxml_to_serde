//! # quickxml_to_serde
//! Fast and flexible conversion from XML to JSON using [quick-xml](https://github.com/tafia/quick-xml)
//! and [serde](https://github.com/serde-rs/json). Inspired by [node2object](https://github.com/vorot93/node2object).
//!
//! This crate converts XML elements, attributes and text nodes into a corresponding JSON structure.
//! Some common usage scenarios would be converting XML into JSON for loading into No-SQL databases
//! or sending it to the front end application.
//!
//! Because of the richness and flexibility of XML some conversion behavior is configurable:
//! - attribute name prefixes
//! - naming of text nodes
//! - number format conversion
//!
//! ## Usage example
//! ```no_run
//! extern crate quickxml_to_serde;
//! use quickxml_to_serde::{xml_string_to_json, Config};
//!
//! fn main() {
//!    let xml = r#"<?xml version="1.0" encoding="utf-8"?><a attr1="1"><b><c attr2="001">some text</c></b></a>"#;
//!    let conf = Config::new_with_defaults();
//!    let json = xml_string_to_json(xml.to_owned(), &conf);
//!    println!("{}", json.expect("Invalid XML").to_string());
//!
//!    let conf = Config::new_with_custom_values(true, "", "txt");
//!    let json = xml_string_to_json(xml.to_owned(), &conf);
//!    println!("{}", json.expect("Invalid XML").to_string());
//! }
//! ```
//! * **Output with default config:** `{"a":{"@attr1":1,"b":{"c":{"#text":"some text","@attr2":1}}}}`
//! * **Output with custom config:** `{"a":{"attr1":1,"b":{"c":{"attr2":"001","txt":"some text"}}}}`
//!
//! ## Detailed documentation
//! See [README](https://github.com/AlecTroemel/quickxml_to_serde) in the source repo for more examples, limitations and detailed behavior description.

extern crate minidom;
extern crate serde_json;

use minidom::{Element, Error};
use serde_json::{Map, Number, Value};
use std::str::FromStr;

/// Tells the converter how to perform certain conversions.
/// See docs for individual fields for more info.
pub struct Config {
    /// Numeric values starting with 0 will be treated as strings.
    /// E.g. `<agent>007</agent>` will become `"agent":"007"`, while
    /// <agent>7</agent>` will become `"agent":7`
    /// Defaults to `false`.
    pub leading_zero_as_string: bool,
    /// Prefix XML attribute names with this value to distinguish them from XML elements.
    /// E.g. set it to `@` for `<x a="Hello!" />` to become `{"x": {"@a":"Hello!"}}`
    /// Defaults to `@`.
    pub xml_attr_prefix: String,
    /// A property name XML text nodes.
    /// E.g. set it to `text` for `<x a="Hello!">Goodbye!</x>` to become `{"x": {"@a":"Hello!", "text":"Goodbye!"}}`
    /// XML nodes with text only and no attributes or no child elements are converted into props with the
    /// name of the element. E.g. <x>Goodbye!</x>` becomes `{"x":"Goodbye!"}`
    /// Defaults to `#text`
    pub xml_text_node_prop_name: String,
}

impl Config {
    /// Numbers with leading zero will be treated as numbers.
    /// Prefix XML Attribute names with `@`
    /// Name XML text nodes `#text` for nodes with other children
    pub fn new_with_defaults() -> Self {
        Config {
            leading_zero_as_string: false,
            xml_attr_prefix: "@".to_owned(),
            xml_text_node_prop_name: "#text".to_owned(),
        }
    }

    /// Create a Config object with non-default values. See the struct docs for more info.
    pub fn new_with_custom_values(
        leading_zero_as_string: bool,
        xml_attr_prefix: &str,
        xml_text_node_prop_name: &str,
    ) -> Self {
        Config {
            leading_zero_as_string,
            xml_attr_prefix: xml_attr_prefix.to_owned(),
            xml_text_node_prop_name: xml_text_node_prop_name.to_owned(),
        }
    }
}

/// Returns the text as one of serde::Value types: int, float, bool or string.
fn parse_text(text: &str, leading_zero_as_string: bool) -> Value {
    let text = text.trim();

    // ints
    if let Ok(v) = text.parse::<u64>() {
        // don't parse octal numbers and those with leading 0
        if text.starts_with("0") && v != 0 && leading_zero_as_string {
            return Value::String(text.into());
        }
        return Value::Number(Number::from(v));
    }

    // floats
    if let Ok(v) = text.parse::<f64>() {
        if text.starts_with("0") && !text.starts_with("0.") {
            return Value::String(text.into());
        }
        if let Some(val) = Number::from_f64(v) {
            return Value::Number(val);
        }
    }

    // booleans
    if let Ok(v) = text.parse::<bool>() {
        return Value::Bool(v);
    }

    Value::String(text.into())
}

fn convert_node(el: &Element, config: &Config) -> Option<Value> {
    if el.text().trim() != "" {
        if el.attrs().count() > 0 {
            Some(Value::Object(
                el.attrs()
                    .map(|(k, v)| {
                        (
                            [config.xml_attr_prefix.clone(), k.to_owned()].concat(),
                            parse_text(&v, config.leading_zero_as_string),
                        )
                    })
                    .chain(vec![(
                        config.xml_text_node_prop_name.clone(),
                        parse_text(&el.text()[..], config.leading_zero_as_string),
                    )])
                    .collect(),
            ))
        } else {
            Some(parse_text(&el.text()[..], config.leading_zero_as_string))
        }
    } else {
        let mut data = Map::new();

        for (k, v) in el.attrs() {
            data.insert(
                [config.xml_attr_prefix.clone(), k.to_owned()].concat(),
                parse_text(&v, config.leading_zero_as_string),
            );
        }

        for child in el.children() {
            match convert_node(child, config) {
                Some(val) => {
                    let name = &child.name().to_string();

                    if data.contains_key(name) {
                        if data.get(name).unwrap_or(&Value::Null).is_array() {
                            data.get_mut(name)
                                .unwrap()
                                .as_array_mut()
                                .unwrap()
                                .push(val);
                        } else {
                            let temp = data.remove(name).unwrap();
                            data.insert(name.clone(), Value::Array(vec![temp, val]));
                        }
                    } else {
                        data.insert(name.clone(), val);
                    }
                }
                _ => (),
            }
        }

        Some(Value::Object(data))
    }
}

fn xml_to_map(e: &Element, config: &Config) -> Value {
    let mut data = Map::new();
    data.insert(
        e.name().to_string(),
        convert_node(&e, &config).unwrap_or(Value::Null),
    );
    Value::Object(data)
}

/// Converts the given XML string into serde::Value using settings from `Config`.
pub fn xml_string_to_json(xml: String, config: &Config) -> Result<Value, Error> {
    let root = Element::from_str(xml.as_str())?;
    Ok(xml_to_map(&root, config))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{json, to_string_pretty};
    use std::fs::File;
    use std::io::prelude::*;

    #[test]
    fn test_numbers() {
        let expected = json!({
            "a": {
                "b":[ 12345, 12345.0, 12345.6 ]
            }
        });
        let result = xml_string_to_json(
            String::from("<a><b>12345</b><b>12345.0</b><b>12345.6</b></a>"),
            &Config::new_with_defaults(),
        );

        assert_eq!(expected, result.unwrap());
    }

    #[test]
    fn test_mixed_nodes() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?><a attr1="val1">some text</a>"#;

        // test with default config values
        let expected_1 = json!({
            "a": {
                "@attr1":"val1",
                "#text":"some text"
            }
        });
        let result_1 = xml_string_to_json(String::from(xml), &Config::new_with_defaults());
        assert_eq!(expected_1, result_1.unwrap());

        // test with custom config values
        let expected_2 = json!({
            "a": {
                "attr1":"val1",
                "text":"some text"
            }
        });
        let conf = Config::new_with_custom_values(true, "", "text");
        let result_2 = xml_string_to_json(String::from(xml), &conf);
        assert_eq!(expected_2, result_2.unwrap());

        // try the same on XML where the attr and children have a name clash
        let xml = r#"<?xml version="1.0" encoding="utf-8"?><a attr1="val1"><attr1><nested>some text</nested></attr1></a>"#;
        let expected_3 = json!({"a":{"attr1":["val1",{"nested":"some text"}]}});

        let result_3 = xml_string_to_json(String::from(xml), &conf);
        assert_eq!(expected_3, result_3.unwrap());
    }

    #[test]
    fn test_malformed_xml() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?><a attr1="val1">some text<b></a>"#;

        let result_1 = xml_string_to_json(String::from(xml), &Config::new_with_defaults());
        assert!(result_1.is_err());
    }

    #[test]
    fn test_parse_text() {
        assert_eq!(0.0, parse_text("0.0", true));
        assert_eq!(0, parse_text("0", true));
        assert_eq!(0.42, parse_text("0.4200", true));
        assert_eq!(142.42, parse_text("142.4200", true));
        assert_eq!("0xAC", parse_text("0xAC", true));
        assert_eq!("0x03", parse_text("0x03", true));
        assert_eq!("142,4200", parse_text("142,4200", true));
        assert_eq!("142,420,0", parse_text("142,420,0", true));
        assert_eq!("142,420,0.0", parse_text("142,420,0.0", true));
        assert_eq!("0Test", parse_text("0Test", true));
        assert_eq!("0.Test", parse_text("0.Test", true));
        assert_eq!("0.22Test", parse_text("0.22Test", true));
        assert_eq!("0044951", parse_text("0044951", true));
        assert_eq!(1, parse_text("1", true));
        assert_eq!(false, parse_text("false", true));
        assert_eq!(true, parse_text("true", true));
        assert_eq!("True", parse_text("True", true));
    }

    #[test]
    fn convert_test_files() {
        // get the list of files in the text directory
        let mut entries = std::fs::read_dir("./test_xml_files")
            .unwrap()
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, std::io::Error>>()
            .unwrap();

        entries.sort();

        let conf = Config::new_with_custom_values(true, "", "text");

        for mut entry in entries {
            // only XML files should be processed
            if entry.extension().unwrap() != "xml" {
                continue;
            }

            // read the XML file
            let mut file = File::open(&entry).unwrap();
            let mut xml_contents = String::new();
            file.read_to_string(&mut xml_contents).unwrap();

            // convert to json
            let json = xml_string_to_json(xml_contents, &conf).unwrap();

            // save as json
            entry.set_extension("json");
            let mut file = File::create(&entry).unwrap();
            assert!(
                file.write_all(to_string_pretty(&json).unwrap().as_bytes())
                    .is_ok(),
                format!("Failed on {:?}", entry.as_os_str())
            );
        }
    }
}
