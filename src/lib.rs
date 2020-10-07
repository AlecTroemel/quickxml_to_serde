extern crate minidom;
extern crate serde_json;

use minidom::{Element, Error};
use serde_json::{Map, Number, Value};
use std::str::FromStr;

/// Tells the converter how to perform certain conversions.
/// See docs for individual elements for more info.
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

fn parse_text(text: &str, config: &Config) -> Value {
    let text = text.trim();

    // ints
    if let Ok(v) = text.parse::<u64>() {
        // don't parse octal numbers and those with leading 0
        if text.starts_with("0") && v != 0 && config.leading_zero_as_string {
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
                    .map(|(k, v)| (format!("@{}", k), parse_text(&v, config)))
                    .chain(vec![(
                        "#text".to_string(),
                        parse_text(&el.text()[..], config),
                    )])
                    .collect(),
            ))
        } else {
            Some(parse_text(&el.text()[..], config))
        }
    } else {
        let mut data = Map::new();

        for (k, v) in el.attrs() {
            data.insert(format!("@{}", k), parse_text(&v, config));
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
    use serde_json::json;

    // #[test]
    // fn map_over_children_test() {
    //     let expected_list = [
    //         (
    //             "<?xml version=\"1.0\" encoding=\"utf-8\"?><b>test1</b>",
    //             json!({"b": "test1"}),
    //         ),
    //         (
    //             "<?xml version=\"1.0\" encoding=\"utf-8\"?><b>test2</b>",
    //             json!({"b": "test2"}),
    //         ),
    //         (
    //             "<?xml version=\"1.0\" encoding=\"utf-8\"?><b>test3</b>",
    //             json!({"b": "test3"}),
    //         ),
    //     ];
    //     let mut expected = expected_list.iter();

    //     map_over_children(
    //         String::from("<a><b>test1</b><b>test2</b><b>test3</b></a>"),
    //         |xml: String, js: Value| {
    //             let expect = expected.next().unwrap();

    //             assert_eq!(expect.0, xml);
    //             assert_eq!(expect.1, js);
    //         },
    //     )
    // }

    // #[test]
    // fn map_of_children_test() {
    //     let expected = vec![
    //         (
    //             String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?><b>test1</b>"),
    //             json!({"b": "test1"}),
    //         ),
    //         (
    //             String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?><b>test2</b>"),
    //             json!({"b": "test2"}),
    //         ),
    //         (
    //             String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?><b>test3</b>"),
    //             json!({"b": "test3"}),
    //         ),
    //     ];
    //     let result = map_of_children(String::from("<a><b>test1</b><b>test2</b><b>test3</b></a>"));
    //     assert_eq!(expected, result);
    // }

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
        println!("{:?}", result);

        // let expected_list = vec![
        //     (
        //         String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?><b>12345</b>"),
        //         json!({"b": 12345}),
        //     ),
        //     (
        //         String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?><b>12345.0</b>"),
        //         json!({"b": 12345.0}),
        //     ),
        //     (
        //         String::from("<?xml version=\"1.0\" encoding=\"utf-8\"?><b>12345.6</b>"),
        //         json!({"b": 12345.6}),
        //     ),
        // ];

        // let result = map_of_children(String::from(
        //     "<a><b>12345</b><b>12345.0</b><b>12345.6</b></a>",
        // ));
        assert_eq!(expected, result.unwrap());
    }

    #[test]
    fn test_parse_text() {
        let config = Config::new_with_custom_values(true, "@", "#text");

        assert_eq!(0.0, parse_text("0.0", &config));
        assert_eq!(0, parse_text("0", &config));
        assert_eq!(0.42, parse_text("0.4200", &config));
        assert_eq!(142.42, parse_text("142.4200", &config));
        assert_eq!("0xAC", parse_text("0xAC", &config));
        assert_eq!("0x03", parse_text("0x03", &config));
        assert_eq!("142,4200", parse_text("142,4200", &config));
        assert_eq!("142,420,0", parse_text("142,420,0", &config));
        assert_eq!("142,420,0.0", parse_text("142,420,0.0", &config));
        assert_eq!("0Test", parse_text("0Test", &config));
        assert_eq!("0.Test", parse_text("0.Test", &config));
        assert_eq!("0.22Test", parse_text("0.22Test", &config));
        assert_eq!("0044951", parse_text("0044951", &config));
        assert_eq!(1, parse_text("1", &config));
        assert_eq!(false, parse_text("false", &config));
        assert_eq!(true, parse_text("true", &config));
        assert_eq!("True", parse_text("True", &config));
    }
}
