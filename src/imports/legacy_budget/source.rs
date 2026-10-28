use super::*;

pub(super) fn read(path: &Path, files: &mut BTreeMap<PathBuf, Vec<u8>>) -> Result<Value> {
    let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let value = serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))?;
    files.insert(path.to_path_buf(), bytes);
    Ok(value)
}
pub(super) fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Missing or invalid {key}"))
}
pub(super) fn optional<'a>(v: &'a Value, key: &str) -> Result<Option<&'a str>> {
    match v.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s)),
        _ => Err(format!("Invalid {key}")),
    }
}
pub(super) fn boolean(v: &Value, key: &str, default: bool) -> Result<bool> {
    match v.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Bool(b)) => Ok(*b),
        _ => Err(format!("Invalid {key}")),
    }
}
pub(super) fn rows<'a>(v: &'a Value, key: &str) -> Result<&'a [Value]> {
    match v.get(key) {
        None | Some(Value::Null) => Ok(&[]),
        Some(Value::Array(a)) => Ok(a),
        _ => Err(format!("Invalid {key}")),
    }
}
pub(super) fn active(v: &Value) -> Result<bool> {
    Ok(!boolean(v, "isTombstone", false)?)
}
pub(super) fn knowledge(s: &str) -> Result<BTreeMap<String, u64>> {
    let mut result = BTreeMap::new();
    for part in s.split(',').filter(|s| !s.is_empty()) {
        let (device, version) = part
            .trim()
            .rsplit_once('-')
            .ok_or("Invalid legacy version knowledge")?;
        let version = version
            .parse()
            .map_err(|_| "Invalid legacy version knowledge")?;
        if device.is_empty() || result.insert(device.into(), version).is_some() {
            return Err("Invalid legacy version knowledge".into());
        }
    }
    Ok(result)
}
pub(super) fn covers(a: &BTreeMap<String, u64>, b: &BTreeMap<String, u64>) -> bool {
    b.iter()
        .all(|(key, n)| a.get(key).copied().unwrap_or(0) >= *n)
}
pub(super) type SourceFiles = BTreeMap<PathBuf, Vec<u8>>;
pub(super) fn load_files(source: &Path) -> Result<(Vec<(PathBuf, Value)>, SourceFiles)> {
    let mut files = BTreeMap::new();
    let mut snapshots = Vec::new();
    if source.is_file() {
        if source.file_name().and_then(|s| s.to_str()) != Some("Budget.yfull") {
            return Err("Choose a legacy budget folder or Budget.yfull snapshot.".into());
        }
        snapshots.push((source.to_path_buf(), read(source, &mut files)?));
    } else {
        let meta = read(&source.join("Budget.ymeta"), &mut files)?;
        if text(&meta, "formatVersion")? != "2" {
            return Err("Unsupported legacy package version".into());
        }
        let relative = Path::new(text(&meta, "relativeDataFolderName")?);
        if relative.components().count() != 1
            || !matches!(
                relative.components().next(),
                Some(std::path::Component::Normal(_))
            )
        {
            return Err("Invalid legacy data folder".into());
        }
        let data = source.join(relative);
        for entry in fs::read_dir(&data).map_err(|e| e.to_string())? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.is_dir() {
                let full = path.join("Budget.yfull");
                if full.is_file() {
                    snapshots.push((full.clone(), read(&full, &mut files)?));
                }
                if path.file_name().and_then(|s| s.to_str()) == Some("devices") {
                    for device in fs::read_dir(&path).map_err(|e| e.to_string())? {
                        let p = device.map_err(|e| e.to_string())?.path();
                        if p.extension().and_then(|s| s.to_str()) == Some("ydevice") {
                            read(&p, &mut files)?;
                        }
                    }
                }
            }
        }
    }
    // Journals beside every snapshot may contain newer edits than the full file.
    for (full, _) in &snapshots {
        for entry in fs::read_dir(full.parent().unwrap()).map_err(|e| e.to_string())? {
            let p = entry.map_err(|e| e.to_string())?.path();
            if p.extension().and_then(|s| s.to_str()) == Some("ydiff") {
                read(&p, &mut files)?;
            }
        }
    }
    if snapshots.is_empty() {
        return Err(
            "No Budget.yfull found. Open, synchronize and close this budget in the source application first."
                .into(),
        );
    }
    Ok((snapshots, files))
}
