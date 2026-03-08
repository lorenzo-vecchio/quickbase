mod common;
use quickbase::models::collection::Collection;
use quickbase::models::record::Record;
use serde_json::json;

fn make_record() -> Record {
    Record::new(Collection::new_base("posts"))
}

#[test]
fn test_new_record_is_new() {
    let r = make_record();
    assert!(r.base.is_new());
}

#[test]
fn test_record_table_name_matches_collection() {
    let r = make_record();
    assert_eq!(r.table_name(), "posts");
}

#[test]
fn test_record_id_is_set() {
    let r = make_record();
    assert!(!r.id().is_empty());
}

#[test]
fn test_record_set_and_get() {
    let mut r = make_record();
    r.set("title", json!("Hello world"));
    assert_eq!(r.get("title"), Some(&json!("Hello world")));
}

#[test]
fn test_record_get_string() {
    let mut r = make_record();
    r.set("title", json!("Hello"));
    assert_eq!(r.get_string("title"), "Hello");
    assert_eq!(r.get_string("missing"), "");
}

#[test]
fn test_record_get_bool() {
    let mut r = make_record();
    r.set("published", json!(true));
    assert!(r.get_bool("published"));
    assert!(!r.get_bool("missing"));
}

#[test]
fn test_record_get_float() {
    let mut r = make_record();
    r.set("price", json!(9.99));
    assert_eq!(r.get_float("price"), 9.99);
    assert_eq!(r.get_float("missing"), 0.0);
}

#[test]
fn test_record_get_int() {
    let mut r = make_record();
    r.set("views", json!(42));
    assert_eq!(r.get_int("views"), 42);
    assert_eq!(r.get_int("missing"), 0);
}

#[test]
fn test_record_load() {
    let mut r = make_record();
    let mut data = std::collections::HashMap::new();
    data.insert("title".to_string(), json!("Loaded title"));
    data.insert("views".to_string(), json!(100));
    r.load(data);
    assert_eq!(r.get_string("title"), "Loaded title");
    assert_eq!(r.get_int("views"), 100);
}

#[test]
fn test_record_expand_empty_by_default() {
    let r = make_record();
    assert!(r.expand.is_empty());
}