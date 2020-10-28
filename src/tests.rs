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
    let xml = r#"<a attr1="007"><b attr1="7" attr2="True">true</b></a>"#;

    // test with default config values
    let expected = json!({
        "a": {
            "@attr1":7,
            "b": {
                "@attr1":7,
                "@attr2":"True",
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
                "@attr2":"True",
            "#text":true
            }
        }
    });
    let conf =
        Config::new_with_defaults().add_json_type_override("/a/@attr1", JsonType::AlwaysString);
    let result = xml_string_to_json(String::from(xml), &conf);
    assert_eq!(expected, result.unwrap());

    // test with custom config values for 3 attributes
    let expected = json!({
        "a": {
            "@attr1":"007",
            "b": {
                "@attr1":"7",
                "@attr2":true,
            "#text":true
            }
        }
    });
    let conf = Config::new_with_defaults()
        .add_json_type_override("/a/@attr1", JsonType::AlwaysString)
        .add_json_type_override("/a/b/@attr1", JsonType::AlwaysString)
        .add_json_type_override("/a/b/@attr2", JsonType::Bool(vec!["True"]));
    let result = xml_string_to_json(String::from(xml), &conf);
    assert_eq!(expected, result.unwrap());

    // test with custom config values for 2 attributes and a text node
    let expected = json!({
        "a": {
            "@attr1":"007",
            "b": {
                "@attr1":"7",
                "@attr2":"True",
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
    assert_eq!(0, parse_text("0", true, &JsonType::Infer));
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

    // always enforce JSON bool type
    let bool_type = JsonType::Bool(vec!["true", "True", "", "1"]);
    assert_eq!(false, parse_text("false", false, &bool_type));
    assert_eq!(true, parse_text("true", false, &bool_type));
    assert_eq!(true, parse_text("True", false, &bool_type));
    assert_eq!(false, parse_text("TRUE", false, &bool_type));
    assert_eq!(true, parse_text("", false, &bool_type));
    assert_eq!(true, parse_text("1", false, &bool_type));
    assert_eq!(false, parse_text("0", false, &bool_type));
    // this is an interesting quirk of &str comparison
    // any whitespace value == "", at least for Vec::contains() fn
    assert_eq!(true, parse_text(" ", false, &bool_type)); 

    // always enforce JSON string type
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
