use super::*;

#[test]
fn sqlite_round_trip_and_invalid_write_preserves_previous() {
    let (b, _, _) = fixture();
    let path = temp("sqlite3");
    {
        let mut s = Store::open(&path).unwrap();
        s.save(&b).unwrap();
        assert_eq!(s.load().unwrap().unwrap().transactions.len(), 2);
        let mut invalid = b.clone();
        invalid.snapshots[0].rate = 0;
        assert!(s.save(&invalid).is_err());
        assert_eq!(s.load().unwrap().unwrap().snapshots[0].rate, 90_000_000);
    }
    std::fs::remove_file(path).unwrap();
}
#[test]
fn json_round_trip_and_tampering_rejected() {
    let (b, _, _) = fixture();
    let path = temp("json");
    export_json(&b, &path).unwrap();
    let restored = import_json(&path).unwrap();
    assert_eq!(restored.snapshots.len(), 2);
    let mut corrupt = restored;
    corrupt.transactions[1].base_amount += 1;
    std::fs::write(&path, serde_json::to_vec(&corrupt).unwrap()).unwrap();
    assert!(import_json(&path).is_err());
    std::fs::remove_file(path).unwrap();
}
