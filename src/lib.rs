//! # quickxml_to_serde
//! Fast and flexible conversion from XML to JSON using [quick-xml](https://github.com/tafia/quick-xml)
//! and [serde](https://github.com/serde-rs/json). Inspired by [node2object](https://github.com/vorot93/node2object).
//!
//! This crate converts XML elements, attributes and text nodes directly into corresponding JSON structures.
//! Some common usage scenarios would be converting XML into JSON for loading into No-SQL databases
//! or sending it to the front end application.
//!
//! Because of the richness and flexibility of XML some conversion behavior is configurable:
//! - attribute name prefixes
//! - naming of text nodes
//! - number format conversion
//!
//! ## Usage example
//! ```
//! extern crate quickxml_to_serde;
//! use quickxml_to_serde::{xml_string_to_json, Config, NullValue};
//!
//! fn main() {
//!    let xml = r#"<a attr1="1"><b><c attr2="001">some text</c></b></a>"#;
//!    let conf = Config::new_with_defaults();
//!    let json = xml_string_to_json(xml.to_owned(), &conf);
//!    println!("{}", json.expect("Malformed XML").to_string());
//!
//!    let conf = Config::new_with_custom_values(true, "", "txt", NullValue::Null);
//!    let json = xml_string_to_json(xml.to_owned(), &conf);
//!    println!("{}", json.expect("Malformed XML").to_string());
//! }
//! ```
//! * **Output with the default config:** `{"a":{"@attr1":1,"b":{"c":{"#text":"some text","@attr2":1}}}}`
//! * **Output with a custom config:** `{"a":{"attr1":1,"b":{"c":{"attr2":"001","txt":"some text"}}}}`
//!
//! ## Additional features
//! Use `quickxml_to_serde = { version = "0.4", features = ["json_types"] }` to enable support for enforcing JSON types     
//! for some XML nodes using xPath-like notations. Example for enforcing attribute `attr2` from the snippet above
//! as JSON String regardless of its contents:
//! ```
//! use quickxml_to_serde::{Config, JsonType};
//!
//! #[cfg(feature = "json_types")]
//! let conf = Config::new_with_defaults().add_json_type_override("/a/b/c/@attr2", JsonType::AlwaysString);
//! ```
//!
//! ## Detailed documentation
//! See [README](https://github.com/AlecTroemel/quickxml_to_serde) in the source repo for more examples, limitations and detailed behavior description.
//!
//! ## Testing your XML files
//!
//! If you want to see how your XML files are converted into JSON, place them into `./test_xml_files` directory
//! and run `cargo test`. They will be converted into JSON and saved in the saved directory.

extern crate minidom;
extern crate serde_json;

use minidom::{Element, Error};
use serde_json::{Map, Number, Value};
#[cfg(feature = "json_types")]
use std::collections::HashMap;
use std::str::FromStr;

/// Defines how empty elements like `<x />` should be handled.
/// `Ignore` -> exclude from JSON, `Null` -> `"x":null`, EmptyObject -> `"x":{}`.
/// `EmptyObject` is the default option and is how it was handled prior to v.0.4
/// Using `Ignore` on an XML document with an empty root element falls back to `Null` option.
/// E.g. both `<a><x/></a>` and `<a/>` are converted into `{"a":null}`.
#[derive(Debug)]
pub enum NullValue {
    Ignore,
    Null,
    EmptyObject,
}

/// Defines which data type to apply in JSON format for consistency of output.
/// E.g., the range of XML values for the same node type may be `1234`, `001234`, `AB1234`.
/// It is impossible to guess with 100% consistency which data type to apply without seeing
/// the entire range of values. Use this enum to tell the converter which data type should
/// be applied.
#[derive(Debug, PartialEq)]
pub enum JsonType {
    /// Do not try to infer the type and convert the value to JSON string.
    /// E.g. convert `<a>1234</a>` into `{"a":"1234"}` or `<a>true</a>` into `{"a":"true"}`
    AlwaysString,
    /// Attempt to infer the type by looking at the single value of the node being converted.
    /// Not guaranteed to be consistent across multiple nodes.
    /// E.g. convert `<a>1234</a>` and `<a>001234</a>` into `{"a":1234}`, or `<a>true</a>` into `{"a":true}`
    /// Check if your values comply with JSON data types (case, range, format) to produce the expected result.
    Infer,
}

/// Tells the converter how to perform certain conversions.
/// See docs for individual fields for more info.
#[derive(Debug)]
pub struct Config {
    /// Numeric values starting with 0 will be treated as strings.
    /// E.g. convert `<agent>007</agent>` into `"agent":"007"` or `"agent":7`
    /// Defaults to `false`.
    pub leading_zero_as_string: bool,
    /// Prefix XML attribute names with this value to distinguish them from XML elements.
    /// E.g. set it to `@` for `<x a="Hello!" />` to become `{"x": {"@a":"Hello!"}}`
    /// or set it to a blank string for `{"x": {"a":"Hello!"}}`
    /// Defaults to `@`.
    pub xml_attr_prefix: String,
    /// A property name for XML text nodes.
    /// E.g. set it to `text` for `<x a="Hello!">Goodbye!</x>` to become `{"x": {"@a":"Hello!", "text":"Goodbye!"}}`
    /// XML nodes with text only and no attributes or no child elements are converted into JSON properties with the
    /// name of the element. E.g. <x>Goodbye!</x>` becomes `{"x":"Goodbye!"}`
    /// Defaults to `#text`
    pub xml_text_node_prop_name: String,
    /// Defines how empty elements like `<x />` should be handled.
    pub empty_element_handling: NullValue,
    /// A list of XML paths with their JsonType overrides. They take precedence over the document-wide `json_type`
    /// property. The path syntax is based on xPath: literal element names and attribute names prefixed with `@`.
    /// The path must start with a leading `/`. It is a bit of an inconvenience to remember about it, but it saves
    /// an extra `if`-check in the code to improve the performance.
    /// # Example
    /// - **XML**: `<a><b c="123">007</b></a>`
    /// - path for `c`: `/a/b/@c`
    /// - path for `b` text node (007): `/a/b`
    #[cfg(feature = "json_types")]
    pub json_type_overrides: HashMap<String, JsonType>,
}

impl Config {
    /// Numbers with leading zero will be treated as numbers.
    /// Prefix XML Attribute names with `@`
    /// Name XML text nodes `#text` for XML Elements with other children
    pub fn new_with_defaults() -> Self {
        Config {
            leading_zero_as_string: false,
            xml_attr_prefix: "@".to_owned(),
            xml_text_node_prop_name: "#text".to_owned(),
            empty_element_handling: NullValue::EmptyObject,
            #[cfg(feature = "json_types")]
            json_type_overrides: HashMap::new(),
        }
    }

    /// Create a Config object with non-default values. See the `Config` struct docs for more info.
    pub fn new_with_custom_values(
        leading_zero_as_string: bool,
        xml_attr_prefix: &str,
        xml_text_node_prop_name: &str,
        empty_element_handling: NullValue,
    ) -> Self {
        Config {
            leading_zero_as_string,
            xml_attr_prefix: xml_attr_prefix.to_owned(),
            xml_text_node_prop_name: xml_text_node_prop_name.to_owned(),
            empty_element_handling,
            #[cfg(feature = "json_types")]
            json_type_overrides: HashMap::new(),
        }
    }

    /// Adds a single JSON Type override rule to the current config.
    /// # Example
    /// - **XML**: `<a><b c="123">007</b></a>`
    /// - path for `c`: `/a/b/@c`
    /// - path for `b` text node (007): `/a/b`
    /// This function will add the leading `/` if it's missing.
    #[cfg(feature = "json_types")]
    pub fn add_json_type_override(self, path: &str, json_type: JsonType) -> Self {
        let mut conf = self;
        let path = if path.starts_with("/") {
            path.to_owned()
        } else {
            ["/", path].concat()
        };
        conf.json_type_overrides.insert(path, json_type);
        conf
    }
}

impl Default for Config {
    fn default() -> Self {
        Config::new_with_defaults()
    }
}

/// Returns the text as one of `serde::Value` types: int, float, bool or string.
fn parse_text(text: &str, leading_zero_as_string: bool, json_type: &JsonType) -> Value {
    let text = text.trim();

    // make it a string regardless of the underlying type
    if json_type == &JsonType::AlwaysString {
        return Value::String(text.into());
    }

    // ints
    if let Ok(v) = text.parse::<u64>() {
        // don't parse octal numbers and those with leading 0
        if text.starts_with("0") && leading_zero_as_string {
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

/// Converts an XML Element into a JSON property
fn convert_node(el: &Element, config: &Config, path: &String) -> Option<Value> {
    // add the current node to the path
    #[cfg(feature = "json_types")]
    let path = [path, "/", el.name()].concat();
    // get the json_type for this node
    #[cfg(feature = "json_types")]
    let json_type = config
        .json_type_overrides
        .get(&path)
        .unwrap_or(&JsonType::Infer);
    #[cfg(not(feature = "json_types"))]
    let json_type = &JsonType::Infer;

    // is it an element with text?
    if el.text().trim() != "" {
        // does it have attributes?
        if el.attrs().count() > 0 {
            Some(Value::Object(
                el.attrs()
                    .map(|(k, v)| {
                        // add the current node to the path
                        #[cfg(feature = "json_types")]
                        let path = [path.clone(), "/@".to_owned(), k.to_owned()].concat();
                        // get the json_type for this node
                        #[cfg(feature = "json_types")]
                        let json_type = config
                            .json_type_overrides
                            .get(&path)
                            .unwrap_or(&JsonType::Infer);
                        (
                            [config.xml_attr_prefix.clone(), k.to_owned()].concat(),
                            parse_text(&v, config.leading_zero_as_string, json_type),
                        )
                    })
                    .chain(vec![(
                        config.xml_text_node_prop_name.clone(),
                        parse_text(&el.text()[..], config.leading_zero_as_string, json_type),
                    )])
                    .collect(),
            ))
        } else {
            Some(parse_text(
                &el.text()[..],
                config.leading_zero_as_string,
                json_type,
            ))
        }
    } else {
        // this element has no text, but may have other child nodes
        let mut data = Map::new();

        for (k, v) in el.attrs() {
            // add the current node to the path
            #[cfg(feature = "json_types")]
            let path = [path.clone(), "/@".to_owned(), k.to_owned()].concat();
            // get the json_type for this node
            #[cfg(feature = "json_types")]
            let json_type = config
                .json_type_overrides
                .get(&path)
                .unwrap_or(&JsonType::Infer);
            data.insert(
                [config.xml_attr_prefix.clone(), k.to_owned()].concat(),
                parse_text(&v, config.leading_zero_as_string, json_type),
            );
        }

        // process child element recursively
        for child in el.children() {
            match convert_node(child, config, &path) {
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

        // return the JSON object if it's not empty
        if !data.is_empty() {
            return Some(Value::Object(data));
        }

        // empty objects are treated according to config rules set by the caller
        match config.empty_element_handling {
            NullValue::Null => Some(Value::Null),
            NullValue::EmptyObject => Some(Value::Object(data)),
            NullValue::Ignore => None,
        }
    }
}

fn xml_to_map(e: &Element, config: &Config) -> Value {
    let mut data = Map::new();
    data.insert(
        e.name().to_string(),
        convert_node(&e, &config, &String::new()).unwrap_or(Value::Null),
    );
    Value::Object(data)
}

/// Converts the given XML string into `serde::Value` using settings from `Config` struct.
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
    fn test_empty_elements_valid() {
        let mut conf = Config::new_with_custom_values(true, "", "text", NullValue::EmptyObject);
        let xml = r#"<a b="1"><x/></a>"#;

        let expected = json!({ "a": {"b":1, "x":{}} });
        let result = xml_string_to_json(xml.to_owned(), &conf);
        assert_eq!(expected, result.unwrap());

        conf.empty_element_handling = NullValue::Null;
        let expected = json!({ "a": {"b":1, "x":null} });
        let result = xml_string_to_json(xml.to_owned(), &conf);
        assert_eq!(expected, result.unwrap());

        conf.empty_element_handling = NullValue::Ignore;
        let expected = json!({ "a": {"b":1} });
        let result = xml_string_to_json(xml.to_owned(), &conf);
        assert_eq!(expected, result.unwrap());
    }

    #[test]
    fn test_empty_elements_invalid() {
        let conf = Config::new_with_custom_values(true, "", "text", NullValue::Ignore);
        let expected = json!({ "a": null });

        let xml = r#"<a><x/></a>"#;
        let result = xml_string_to_json(xml.to_owned(), &conf);
        assert_eq!(expected, result.unwrap());

        let xml = r#"<a />"#;
        let result = xml_string_to_json(xml.to_owned(), &conf);
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
        let conf = Config::new_with_custom_values(true, "", "text", NullValue::Null);
        let result_2 = xml_string_to_json(String::from(xml), &conf);
        assert_eq!(expected_2, result_2.unwrap());

        // try the same on XML where the attr and children have a name clash
        let xml = r#"<?xml version="1.0" encoding="utf-8"?><a attr1="val1"><attr1><nested>some text</nested></attr1></a>"#;
        let expected_3 = json!({"a":{"attr1":["val1",{"nested":"some text"}]}});

        let result_3 = xml_string_to_json(String::from(xml), &conf);
        assert_eq!(expected_3, result_3.unwrap());
    }

    #[cfg(feature = "json_types")]
    #[test]
    fn test_add_json_type_override() {
        // check if it adds the leading slash
        let config =
            Config::new_with_defaults().add_json_type_override("a/@attr1", JsonType::AlwaysString);
        assert!(config.json_type_overrides.get("/a/@attr1").is_some());

        // check if it doesn't add any extra slashes
        let config =
            Config::new_with_defaults().add_json_type_override("/a/@attr1", JsonType::AlwaysString);
        assert!(config.json_type_overrides.get("/a/@attr1").is_some());
    }

    #[cfg(feature = "json_types")]
    #[test]
    fn test_json_type_overrides() {
        let xml = r#"<a attr1="007"><b attr1="7">true</b></a>"#;

        // test with default config values
        let expected = json!({
            "a": {
                "@attr1":7,
                "b": {
                    "@attr1":7,
                "#text":true
                }
            }
        });
        let config = Config::new_with_defaults();
        let result = xml_string_to_json(String::from(xml), &config);
        assert_eq!(expected, result.unwrap());

        // test with custom config values for 1 attribute
        let expected = json!({
            "a": {
                "@attr1":"007",
                "b": {
                    "@attr1":7,
                "#text":true
                }
            }
        });
        let conf =
            Config::new_with_defaults().add_json_type_override("/a/@attr1", JsonType::AlwaysString);
        let result = xml_string_to_json(String::from(xml), &conf);
        assert_eq!(expected, result.unwrap());

        // test with custom config values for 2 attributes
        let expected = json!({
            "a": {
                "@attr1":"007",
                "b": {
                    "@attr1":"7",
                "#text":true
                }
            }
        });
        let conf = Config::new_with_defaults()
            .add_json_type_override("/a/@attr1", JsonType::AlwaysString)
            .add_json_type_override("/a/b/@attr1", JsonType::AlwaysString);
        let result = xml_string_to_json(String::from(xml), &conf);
        assert_eq!(expected, result.unwrap());

        // test with custom config values for 2 attributes and a text node
        let expected = json!({
            "a": {
                "@attr1":"007",
                "b": {
                    "@attr1":"7",
                "#text":"true"
                }
            }
        });
        let conf = Config::new_with_defaults()
            .add_json_type_override("/a/@attr1", JsonType::AlwaysString)
            .add_json_type_override("/a/b/@attr1", JsonType::AlwaysString)
            .add_json_type_override("/a/b", JsonType::AlwaysString);
        let result = xml_string_to_json(String::from(xml), &conf);
        assert_eq!(expected, result.unwrap());
    }

    #[test]
    fn test_malformed_xml() {
        let xml = r#"<?xml version="1.0" encoding="utf-8"?><a attr1="val1">some text<b></a>"#;

        let result_1 = xml_string_to_json(String::from(xml), &Config::new_with_defaults());
        assert!(result_1.is_err());
    }

    #[test]
    fn test_parse_text() {
        assert_eq!(0.0, parse_text("0.0", false, &JsonType::Infer));
        assert_eq!(0, parse_text("0", false, &JsonType::Infer));
        assert_eq!(0, parse_text("0000", false, &JsonType::Infer));
        assert_eq!("0", parse_text("0", true, &JsonType::Infer));
        assert_eq!("0000", parse_text("0000", true, &JsonType::Infer));
        assert_eq!(0.42, parse_text("0.4200", false, &JsonType::Infer));
        assert_eq!(142.42, parse_text("142.4200", false, &JsonType::Infer));
        assert_eq!("0xAC", parse_text("0xAC", true, &JsonType::Infer));
        assert_eq!("0x03", parse_text("0x03", true, &JsonType::Infer));
        assert_eq!("142,4200", parse_text("142,4200", true, &JsonType::Infer));
        assert_eq!("142,420,0", parse_text("142,420,0", true, &JsonType::Infer));
        assert_eq!(
            "142,420,0.0",
            parse_text("142,420,0.0", true, &JsonType::Infer)
        );
        assert_eq!("0Test", parse_text("0Test", true, &JsonType::Infer));
        assert_eq!("0.Test", parse_text("0.Test", true, &JsonType::Infer));
        assert_eq!("0.22Test", parse_text("0.22Test", true, &JsonType::Infer));
        assert_eq!("0044951", parse_text("0044951", true, &JsonType::Infer));
        assert_eq!(1, parse_text("1", true, &JsonType::Infer));
        assert_eq!(false, parse_text("false", false, &JsonType::Infer));
        assert_eq!(true, parse_text("true", true, &JsonType::Infer));
        assert_eq!("True", parse_text("True", true, &JsonType::Infer));
        // always enforce string JSON type
        assert_eq!("abc", parse_text("abc", false, &JsonType::AlwaysString));
        assert_eq!("true", parse_text("true", false, &JsonType::AlwaysString));
        assert_eq!("123", parse_text("123", false, &JsonType::AlwaysString));
        assert_eq!("0123", parse_text("0123", false, &JsonType::AlwaysString));
        assert_eq!(
            "0.4200",
            parse_text("0.4200", false, &JsonType::AlwaysString)
        );
    }

    /// A shortcut for testing the conversion using XML files.
    /// Place your XML files in `./test_xml_files` directory and run `cargo test`.
    /// They will be converted into JSON and saved in the saved directory.
    #[test]
    fn convert_test_files() {
        // get the list of files in the text directory
        let mut entries = std::fs::read_dir("./test_xml_files")
            .unwrap()
            .map(|res| res.map(|e| e.path()))
            .collect::<Result<Vec<_>, std::io::Error>>()
            .unwrap();

        entries.sort();

        let conf = Config::new_with_custom_values(true, "", "text", NullValue::Null);

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
