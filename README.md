# quickxml_to_serde

Convert XML to JSON using quickxml and serde. Inspired by [node2object](https://github.com/vorot93/node2object).

## Usage examples

#### Basic

```
use std::fs::File;
use std::io::prelude::*;
use quickxml_to_serde::xml_string_to_json;

...

// read an XML file into a string
let mut xml_file = File::open("test.xml")?;
let mut xml_contents = String::new();
xml_file.read_to_string(&mut xml_contents)?;

// convert the XML string into JSON with default config params
let json = xml_string_to_json(xml_contents, &Config::new_with_defaults());

println!("{}", json);
```

#### Custom config

The following config example changes the default behavior to:

1. Treat numbers starting with `0` as strings. E.g. `007` will be `"007"`
2. Do not prefix properties created from attributes
3. Use `text` as the property name for values of XML text nodes where the text is mixed with other nodes

```
let conf = Config::new_with_custom_values(true, "", "text");
```

See embedded docs for `Config` struct for more details. 

## Conversion specifics

- The order of XML elements is not preserved
- Namespace identifiers are dropped. E.g. `<xs:a>123</xs:a>` becomes `"a":123`
- Integers and floats are converted into JSON integers and floats. See `Config` members for some fine-tuning.
- Attributes become properties at the same level as child elements. E.g.
```
<Test TestId="0001">
  <Input>1</Input>
</Test>
```
is converted into
```
"Test":
  {
    "Input": 1,
    "TestId": "0001"
  }
```
- XML prolog is dropped. E.g. `<?xml version="1.0"?>`.
- XML namespace definitions are dropped. E.g. `<Tests xmlns="http://www.adatum.com" />` becomes `"Tests":{}`
- Processing instructions, comments and DTD are ignored
- **Presence of CDATA results in incorrect JSON**
- Attributes can be prefixed via `Config::xml_attr_prefix`. E.g. using the default prefix `@` converts `<a b="y">` into `"a": {"@b":"y"}`. You can use no prefix or set your own value. 
- Complex elements with text nodes put the text node value into a property named in `Config::xml_text_node_prop_name`. E.g. setting `xml_text_node_prop_name` to `text` will convert
```<CardNumber Month="3" Year="19">134567/CardNumber>```
into
```
"CardNumber": {
        "Month": 3,
        "Year": 19,
        "text": 1342260649
      }
```
- Elements with identical names are collected into arrays. E.g.
```
<Root>
  <TaxRate>7.25</TaxRate>
  <Data>
    <Category>A</Category>
    <Quantity>3</Quantity>
    <Price>24.50</Price>
  </Data>
  <Data>
    <Category>B</Category>
    <Quantity>1</Quantity>
    <Price>89.99</Price>
  </Data>
</Root>
```
converts into
```
{
  "Root": {
    "Data": [
      {
        "Category": "A",
        "Price": 24.5,
        "Quantity": 3
      },
      {
        "Category": "B",
        "Price": 89.99,
        "Quantity": 1
      }
    ],
    "TaxRate": 7.25
  }
}
```
-  If `TaxRate` element from the above example was inserted between `Data` elements it would still produce the same JSON with all `Data` properties grouped into a single array.

#### Additional info and examples

See `mod tests` inside [lib.rs](src/lib.rs) for more usage examples.