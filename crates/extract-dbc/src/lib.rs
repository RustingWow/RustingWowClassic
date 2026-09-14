use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, bail};
use wow_mpq::PatchChain;

const VANILLA_LOCALES: &[&str] = &[
    "enUS", "enGB", "deDE", "esES", "esMX", "frFR", "koKR", "ruRU", "zhCN", "zhTW",
];

/// Resolve the client's `Data/` directory from a WoW install or `Data` path.
pub fn data_dir(client: &Path) -> anyhow::Result<PathBuf> {
    if client.join("Data").is_dir() {
        return Ok(client.join("Data"));
    }
    if client
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case("Data"))
        && client.is_dir()
    {
        return Ok(client.to_path_buf());
    }
    bail!(
        "no Data/ directory under {} (pass the 1.12.1 install folder or Data/)",
        client.display()
    );
}

/// Locale from `--locale`, or the first `Data/<locale>/locale-<locale>.MPQ` found.
///
/// Retail 1.12 keeps DBC in locale MPQs. Retro-style clients (e.g. RetroWoW) put
/// them in `dbc.MPQ` with no locale folder — then this returns `None`.
pub fn detect_locale(data: &Path, requested: Option<&str>) -> anyhow::Result<Option<String>> {
    if let Some(locale) = requested {
        let path = locale_mpq(data, locale);
        if path.is_file() {
            return Ok(Some(locale.to_string()));
        }
        bail!("locale {locale} not found (expected {})", path.display());
    }
    for locale in VANILLA_LOCALES {
        if locale_mpq(data, locale).is_file() {
            return Ok(Some((*locale).to_string()));
        }
    }
    if let Some(locale) = scan_locale_dirs(data) {
        return Ok(Some(locale));
    }
    Ok(None)
}

fn locale_mpq(data: &Path, locale: &str) -> PathBuf {
    data.join(locale).join(format!("locale-{locale}.MPQ"))
}

fn scan_locale_dirs(data: &Path) -> Option<String> {
    let entries = fs::read_dir(data).ok()?;
    for entry in entries.flatten() {
        if !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name();
        let locale = name.to_string_lossy();
        if locale_mpq(data, &locale).is_file() {
            return Some(locale.into_owned());
        }
    }
    None
}

/// Vanilla patch chain: present archives only, lowest priority first.
///
/// Retail uses `common.MPQ` + locale MPQs. RetroWoW-style installs ship a
/// dedicated `dbc.MPQ` (and often `base.MPQ`) instead of locale folders.
pub fn vanilla_archives(data: &Path, locale: Option<&str>) -> Vec<(PathBuf, i32)> {
    let mut candidates = vec![
        (data.join("dbc.MPQ"), 0),
        (data.join("common.MPQ"), 10),
        (data.join("base.MPQ"), 20),
        (data.join("backup.MPQ"), 30),
        (data.join("patch.MPQ"), 100),
        (data.join("patch-2.MPQ"), 200),
        (data.join("patch-3.MPQ"), 300),
    ];
    if let Some(locale) = locale {
        let locale_dir = data.join(locale);
        candidates.extend([
            (locale_dir.join(format!("locale-{locale}.MPQ")), 400),
            (locale_dir.join(format!("patch-{locale}.MPQ")), 500),
            (locale_dir.join(format!("patch-{locale}-2.MPQ")), 600),
            (locale_dir.join(format!("patch-{locale}-3.MPQ")), 700),
        ]);
    }
    candidates
        .into_iter()
        .filter(|(path, _)| path.is_file())
        .collect()
}

/// If `archive_path` is `DBFilesClient/*.dbc`, return the file name to write.
pub fn dbc_output_name(archive_path: &str) -> Option<String> {
    let normalized = archive_path.replace('\\', "/");
    let (parent, file) = normalized.rsplit_once('/')?;
    let folder = parent.rsplit('/').next()?;
    if folder.eq_ignore_ascii_case("DBFilesClient") && file.to_ascii_lowercase().ends_with(".dbc") {
        Some(file.to_string())
    } else {
        None
    }
}

/// Extract every `DBFilesClient/*.dbc` from the Vanilla MPQ chain into `out`.
pub fn extract_dbcs(client: &Path, out: &Path, locale: Option<&str>) -> anyhow::Result<usize> {
    let data = data_dir(client)?;
    let locale = detect_locale(&data, locale)?;
    let archives = vanilla_archives(&data, locale.as_deref());
    if archives.is_empty() {
        bail!(
            "no DBC MPQ archives under {} (need dbc.MPQ, common.MPQ, or locale-*.MPQ)",
            data.display()
        );
    }

    tracing::info!(
        data = %data.display(),
        locale = locale.as_deref().unwrap_or("none"),
        archives = archives.len(),
        "opening client MPQ chain"
    );

    let mut chain = PatchChain::new();
    for (path, priority) in &archives {
        chain
            .add_archive(path, *priority)
            .with_context(|| format!("open {}", path.display()))?;
        tracing::info!(file = %path.display(), priority, "added MPQ");
    }

    fs::create_dir_all(out).with_context(|| format!("create {}", out.display()))?;

    let entries = chain.list().context("list MPQ files")?;
    let mut names = Vec::new();
    for entry in &entries {
        if let Some(name) = dbc_output_name(entry_name(entry)) {
            names.push((entry_name(entry).to_string(), name));
        }
    }
    names.sort_by(|a, b| a.1.to_ascii_lowercase().cmp(&b.1.to_ascii_lowercase()));
    names.dedup_by(|a, b| a.1.eq_ignore_ascii_case(&b.1));

    let mut written = 0usize;
    for (archive_name, file_name) in names {
        let bytes = chain
            .read_file(&archive_name)
            .with_context(|| format!("read {archive_name}"))?;
        let dest = out.join(&file_name);
        fs::write(&dest, bytes).with_context(|| format!("write {}", dest.display()))?;
        written += 1;
    }
    Ok(written)
}

fn entry_name(entry: &wow_mpq::FileEntry) -> &str {
    &entry.name
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn dbc_output_name_accepts_slash_and_backslash() {
        assert_eq!(
            dbc_output_name(r"DBFilesClient\FactionTemplate.dbc").as_deref(),
            Some("FactionTemplate.dbc")
        );
        assert_eq!(
            dbc_output_name("DBFilesClient/Spell.dbc").as_deref(),
            Some("Spell.dbc")
        );
        assert_eq!(dbc_output_name("Interface/Icons/foo.blp"), None);
        assert_eq!(dbc_output_name("DBFilesClient/readme.txt"), None);
    }

    #[test]
    fn discovers_data_dir_and_locale_from_fake_client() {
        let root =
            std::env::temp_dir().join(format!("wowserver-extract-dbc-{}", std::process::id()));
        let data = root.join("Data");
        let locale_dir = data.join("enUS");
        fs::create_dir_all(&locale_dir).unwrap();
        fs::write(data.join("common.MPQ"), []).unwrap();
        fs::write(data.join("patch.MPQ"), []).unwrap();
        fs::write(locale_dir.join("locale-enUS.MPQ"), []).unwrap();
        fs::write(locale_dir.join("patch-enUS.MPQ"), []).unwrap();

        let found = data_dir(&root).unwrap();
        assert_eq!(found, data);
        assert_eq!(data_dir(&data).unwrap(), data);
        assert_eq!(detect_locale(&data, None).unwrap().as_deref(), Some("enUS"));
        assert_eq!(
            detect_locale(&data, Some("enUS")).unwrap().as_deref(),
            Some("enUS")
        );

        let archives = vanilla_archives(&data, Some("enUS"));
        let names: Vec<_> = archives
            .iter()
            .map(|(path, priority)| {
                (
                    path.file_name().unwrap().to_string_lossy().into_owned(),
                    *priority,
                )
            })
            .collect();
        assert_eq!(
            names,
            [
                ("common.MPQ".into(), 10),
                ("patch.MPQ".into(), 100),
                ("locale-enUS.MPQ".into(), 400),
                ("patch-enUS.MPQ".into(), 500),
            ]
        );

        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn retro_layout_uses_dbc_mpq_without_locale() {
        let root = std::env::temp_dir().join(format!(
            "wowserver-extract-dbc-retro-{}",
            std::process::id()
        ));
        let data = root.join("Data");
        fs::create_dir_all(&data).unwrap();
        fs::write(data.join("dbc.MPQ"), []).unwrap();
        fs::write(data.join("base.MPQ"), []).unwrap();
        fs::write(data.join("patch.MPQ"), []).unwrap();
        fs::write(data.join("patch-2.MPQ"), []).unwrap();

        assert_eq!(detect_locale(&data, None).unwrap(), None);
        let names: Vec<_> = vanilla_archives(&data, None)
            .iter()
            .map(|(path, priority)| {
                (
                    path.file_name().unwrap().to_string_lossy().into_owned(),
                    *priority,
                )
            })
            .collect();
        assert_eq!(
            names,
            [
                ("dbc.MPQ".into(), 0),
                ("base.MPQ".into(), 20),
                ("patch.MPQ".into(), 100),
                ("patch-2.MPQ".into(), 200),
            ]
        );

        let _ = fs::remove_dir_all(&root);
    }
}
