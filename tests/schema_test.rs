use quickbase::models::schema::{SchemaField, FieldType};

#[test]
fn test_field_type_display() {
    assert_eq!(FieldType::Text.to_string(), "text");
    assert_eq!(FieldType::Number.to_string(), "number");
    assert_eq!(FieldType::Bool.to_string(), "bool");
    assert_eq!(FieldType::Date.to_string(), "date");
    assert_eq!(FieldType::Json.to_string(), "json");
    assert_eq!(FieldType::Email.to_string(), "email");
    assert_eq!(FieldType::Url.to_string(), "url");
}

#[test]
fn test_field_type_from_str() {
    use std::str::FromStr;
    assert_eq!(FieldType::from_str("text").unwrap(), FieldType::Text);
    assert_eq!(FieldType::from_str("number").unwrap(), FieldType::Number);
    assert_eq!(FieldType::from_str("bool").unwrap(), FieldType::Bool);
    assert_eq!(FieldType::from_str("date").unwrap(), FieldType::Date);
    assert_eq!(FieldType::from_str("json").unwrap(), FieldType::Json);
    assert_eq!(FieldType::from_str("email").unwrap(), FieldType::Email);
    assert_eq!(FieldType::from_str("url").unwrap(), FieldType::Url);
    assert!(FieldType::from_str("unknown").is_err());
}

#[test]
fn test_schema_field_new() {
    let field = SchemaField::new("title", FieldType::Text);
    assert_eq!(field.name, "title");
    assert_eq!(field.field_type, FieldType::Text);
    assert!(!field.required);
    assert!(field.id.starts_with('f'));
}

#[test]
fn test_schema_field_sqlite_type() {
    assert_eq!(SchemaField::new("title", FieldType::Text).sqlite_type(), "TEXT");
    assert_eq!(SchemaField::new("views", FieldType::Number).sqlite_type(), "REAL");
    assert_eq!(SchemaField::new("active", FieldType::Bool).sqlite_type(), "INTEGER");
    assert_eq!(SchemaField::new("created_at", FieldType::Date).sqlite_type(), "TEXT");
    assert_eq!(SchemaField::new("meta", FieldType::Json).sqlite_type(), "TEXT");
    assert_eq!(SchemaField::new("email", FieldType::Email).sqlite_type(), "TEXT");
    assert_eq!(SchemaField::new("website", FieldType::Url).sqlite_type(), "TEXT");
}

#[test]
fn test_schema_field_serialization() {
    let field = SchemaField::new("title", FieldType::Text);
    let json = serde_json::to_value(&field).unwrap();
    assert_eq!(json["name"], "title");
    assert_eq!(json["type"], "text");
    assert_eq!(json["required"], false);
}

#[test]
fn test_schema_field_deserialization() {
    let json = serde_json::json!({
        "id": "f00000001",
        "name": "views",
        "type": "number",
        "required": true,
        "options": {}
    });
    let field: SchemaField = serde_json::from_value(json).unwrap();
    assert_eq!(field.name, "views");
    assert_eq!(field.field_type, FieldType::Number);
    assert!(field.required);
}