use std::collections::HashSet;
use quickbase::tools::security::generate_id;

#[test]
fn test_generate_id_length() {
    let id = generate_id();
    assert_eq!(id.len(), 15);
}

#[test]
fn test_generate_id_alphanumeric() {
    let id = generate_id();
    assert!(id.chars().all(|c| c.is_alphanumeric()));
}

#[test]
fn test_generate_id_uniqueness() {
    let ids: HashSet<String> = (0..1000).map(|_| generate_id()).collect();
    assert_eq!(ids.len(), 1000);
}