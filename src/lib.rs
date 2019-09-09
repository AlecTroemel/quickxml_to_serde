extern crate minidom;
extern crate quick_xml;

#[macro_use]
extern crate serde_json;

use minidom::Element;
use quick_xml::Reader;
use serde_json::{Map, Number, Value};
use std::io::BufRead;
use std::str::FromStr;

#[derive(Debug, Copy, Clone)]
pub struct Config {
    leading_zero_as_string: bool,
}

fn parse_text(text: &str, config: Config) -> Value {
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

fn convert_node(el: &Element, config: Config) -> Option<Value> {
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

pub fn xml_to_map(e: &Element, config: Option<Config>) -> Value {
    let config = match config {
        Some(c) => c,
        None => Config {
            leading_zero_as_string: false,
        },
    };

    let mut data = Map::new();
    data.insert(
        e.name().to_string(),
        convert_node(&e, config).unwrap_or(Value::Null),
    );
    Value::Object(data)
}

pub fn xml_string_to_json(xml: String, config: Option<Config>) -> Value {
    let root = Element::from_str(xml.as_str()).unwrap();
    xml_to_map(&root, config)
}

pub fn map_over_children<T: BufRead, F: FnMut(&str, &Value)>(
    xml: T,
    mut iteratee: F,
    config: Option<Config>,
) {
    let mut reader = Reader::from_reader(xml);
    let root = Element::from_reader(&mut reader).unwrap();

    for child in root.children() {
        let mut child_xml = Vec::new();
        child
            .write_to(&mut child_xml)
            .expect("successfully write to the vector");
        let xml_string = String::from_utf8(child_xml).unwrap();
        iteratee(xml_string.as_str(), &xml_to_map(&child, config));
    }
}

pub fn map_of_children(root: Element, config: Option<Config>) -> Vec<(String, Value)> {
    root.children()
        .map(|child| {
            let mut child_xml = Vec::new();
            child
                .write_to(&mut child_xml)
                .expect("successfully write to the vector");
            (
                String::from_utf8(child_xml).unwrap(),
                xml_to_map(&child, config),
            )
        })
        .collect()
}

pub fn get_root<T: BufRead>(xml: T) -> Result<Element, minidom::Error> {
    let mut reader = Reader::from_reader(xml);
    Element::from_reader(&mut reader)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

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
            None,
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
        assert_eq!(expected, result);
    }

    #[test]
    fn test_parse_text() {
        let config = Config {
            leading_zero_as_string: true,
        };

        assert_eq!(0.0, parse_text("0.0", config));
        assert_eq!(0, parse_text("0", config));
        assert_eq!(0.42, parse_text("0.4200", config));
        assert_eq!(142.42, parse_text("142.4200", config));
        assert_eq!("0xAC", parse_text("0xAC", config));
        assert_eq!("0x03", parse_text("0x03", config));
        assert_eq!("142,4200", parse_text("142,4200", config));
        assert_eq!("142,420,0", parse_text("142,420,0", config));
        assert_eq!("142,420,0.0", parse_text("142,420,0.0", config));
        assert_eq!("0Test", parse_text("0Test", config));
        assert_eq!("0.Test", parse_text("0.Test", config));
        assert_eq!("0.22Test", parse_text("0.22Test", config));
        assert_eq!("0044951", parse_text("0044951", config));
        assert_eq!(1, parse_text("1", config));
        assert_eq!(false, parse_text("false", config));
        assert_eq!(true, parse_text("true", config));
        assert_eq!("True", parse_text("True", config));
    }
}
