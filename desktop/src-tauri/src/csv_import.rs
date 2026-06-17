use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportRow {
    pub title: String,
    pub url: String,
    pub username: String,
    pub password: String,
}

enum Fmt {
    Chrome,  // name, url, username, password
    Firefox, // url, username, password, httpRealm, ...
}

/// Parse a CSV export from Chrome or Firefox.
/// Automatically detects the format by inspecting the header row.
/// Strips a UTF-8 BOM if present (Chrome adds one on Windows).
pub fn parse(raw: &str) -> Result<Vec<ImportRow>, String> {
    let content = raw.strip_prefix('\u{feff}').unwrap_or(raw);

    let mut rdr = csv::ReaderBuilder::new()
        .flexible(true)
        .from_reader(content.as_bytes());

    let headers = rdr.headers().map_err(|e| e.to_string())?.clone();
    let fmt = detect_format(&headers)?;

    let mut rows = Vec::new();
    for result in rdr.records() {
        let rec = result.map_err(|e| e.to_string())?;
        let row = match fmt {
            Fmt::Chrome => ImportRow {
                title:    rec.get(0).unwrap_or("").trim().to_owned(),
                url:      rec.get(1).unwrap_or("").trim().to_owned(),
                username: rec.get(2).unwrap_or("").trim().to_owned(),
                password: rec.get(3).unwrap_or("").trim().to_owned(),
            },
            Fmt::Firefox => {
                let url = rec.get(0).unwrap_or("").trim().to_owned();
                ImportRow {
                    title:    derive_title(&url),
                    url,
                    username: rec.get(1).unwrap_or("").trim().to_owned(),
                    password: rec.get(2).unwrap_or("").trim().to_owned(),
                }
            }
        };
        // Skip rows where both url and username are empty (blank lines etc.)
        if !row.url.is_empty() || !row.username.is_empty() {
            rows.push(row);
        }
    }

    Ok(rows)
}

fn detect_format(headers: &csv::StringRecord) -> Result<Fmt, String> {
    let cols: Vec<String> = headers.iter().map(|s| s.to_lowercase()).collect();
    match cols.first().map(|s| s.as_str()) {
        Some("name") => Ok(Fmt::Chrome),
        Some("url") => Ok(Fmt::Firefox),
        other => Err(format!(
            "Unknown CSV format. Expected first column to be 'name' (Chrome) or 'url' (Firefox), got: {:?}",
            other
        )),
    }
}

fn derive_title(url: &str) -> String {
    url.trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .unwrap_or(url)
        .to_owned()
}
