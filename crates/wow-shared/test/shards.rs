use super::*;

#[test]
fn builtin_covers_both_continents() {
    let file = ShardFile::builtin();
    assert_eq!(file.maps(), vec![MAP_EASTERN_KINGDOMS, MAP_KALIMDOR]);
    assert_eq!(
        file.shard_for_map(MAP_EASTERN_KINGDOMS).unwrap().name,
        "eastern-kingdoms"
    );
}

#[test]
fn parse_rejects_duplicate_map() {
    let yaml = r#"
shards:
  - name: a
    maps: [0]
  - name: b
    maps: [0]
"#;
    assert!(ShardFile::parse(yaml).is_err());
}

#[test]
fn parse_instance_kind() {
    let yaml = r#"
shards:
  - name: eastern-kingdoms
    maps: [0]
    dedicated: true
  - name: instances
    maps: [36, 389]
    kind: instance
"#;
    let file = ShardFile::parse(yaml).unwrap();
    assert_eq!(file.shards[1].kind, ShardKind::Instance);
    assert!(!file.shards[1].dedicated);
}
