mod common;
use quickbase::models::collection::{Collection, CollectionType};

#[test]
fn test_new_base_collection() {
    let c = Collection::new_base("posts");
    assert_eq!(c.name, "posts");
    assert!(c.is_base());
    assert!(!c.is_auth());
    assert!(!c.is_view());
    assert!(c.base.is_new());
}

#[test]
fn test_new_auth_collection() {
    let c = Collection::new_auth("users");
    assert_eq!(c.collection_type, CollectionType::Auth);
    assert!(c.is_auth());
}

#[test]
fn test_new_view_collection() {
    let c = Collection::new_view("posts_view");
    assert_eq!(c.collection_type, CollectionType::View);
    assert!(c.is_view());
}

#[test]
fn test_collection_type_display() {
    assert_eq!(CollectionType::Base.to_string(), "base");
    assert_eq!(CollectionType::Auth.to_string(), "auth");
    assert_eq!(CollectionType::View.to_string(), "view");
}

#[test]
fn test_collection_default_rules_are_none() {
    let c = Collection::new_base("articles");
    assert!(c.list_rule.is_none());
    assert!(c.view_rule.is_none());
    assert!(c.create_rule.is_none());
    assert!(c.update_rule.is_none());
    assert!(c.delete_rule.is_none());
}

#[test]
fn test_collection_default_schema_is_empty_array() {
    let c = Collection::new_base("articles");
    assert!(c.schema.is_empty());
    assert_eq!(*c.indexes, serde_json::json!([]));
}

#[test]
fn test_collection_table_name() {
    assert_eq!(Collection::table_name(), "_collections");
}

#[test]
fn test_collection_id_is_set() {
    let c = Collection::new_base("test");
    assert!(!c.id().is_empty());
}