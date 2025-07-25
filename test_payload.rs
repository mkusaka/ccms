use tuirealm::props::{AttrValue, PropPayload, PropValue};

fn main() {
    // Test if Payload variant exists
    let test_payload = AttrValue::Payload(PropPayload::One(PropValue::Str("test".to_string())));
    
    match test_payload {
        AttrValue::Payload(payload) => {
            println!("Payload variant exists!");
            match payload {
                PropPayload::One(PropValue::Str(s)) => println!("String value: {}", s),
                _ => println!("Other payload type"),
            }
        }
        _ => println!("Not a payload"),
    }
    
    // Test with Vec
    let vec_payload = AttrValue::Payload(PropPayload::Vec(vec![
        PropPayload::One(PropValue::Str("item1".to_string())),
        PropPayload::One(PropValue::Str("item2".to_string())),
    ]));
    
    println!("Vec payload created successfully");
    
    // Test with Map
    let mut map = std::collections::HashMap::new();
    map.insert("key1".to_string(), PropValue::Str("value1".to_string()));
    map.insert("key2".to_string(), PropValue::U32(42));
    
    let map_payload = AttrValue::Payload(PropPayload::Map(map));
    println!("Map payload created successfully");
}