extern crate minidom;
extern crate quick_xml;
extern crate serde_json;

use minidom::Element;
use quick_xml::Reader;
use serde_json::{Map, Number, Value};

fn parse_text(text: &str) -> Value {
    match text.parse::<f64>() {
        Ok(v) => match Number::from_f64(v) {
            Some(v) => {
                return Value::Number(v);
            }
            _ => {}
        },
        _ => {}
    }

    match text.parse::<bool>() {
        Ok(v) => {
            return Value::Bool(v);
        }
        _ => {}
    }

    Value::String(text.into())
}

fn convert_node(el: &Element) -> Option<Value> {
    if el.text().trim() != "" {
        if el.attrs().count() > 0 {
            Some(Value::Object(
                el.attrs()
                    .map(|(k, v)| (format!("@{}", k), parse_text(&v)))
                    .chain(vec![("#text".to_string(), parse_text(&el.text()[..]))])
                    .collect(),
            ))
        } else {
            Some(parse_text(&el.text()[..]))
        }
    } else {
        let mut data = Map::new();

        for (k, v) in el.attrs() {
            data.insert(format!("@{}", k), parse_text(&v));
        }

        for child in el.children() {
            match convert_node(child) {
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

pub fn xml_to_map(e: &Element) -> Value {
    let mut data = Map::new();
    data.insert(
        e.name().to_string(),
        convert_node(&e).unwrap_or(Value::Null),
    );
    Value::Object(data)
}

pub fn xml_string_to_json(xml: String) -> Value {
    let mut reader = Reader::from_str(xml.as_str());//.expect("failed loading str into reader");
    let root = Element::from_reader(&mut reader).expect("failed loading into minidom");
    xml_to_map(&root)
}
