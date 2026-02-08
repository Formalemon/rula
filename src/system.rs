// ============================================================================
// System Scanner - Optimized with Caching and Lazy Loading
// ============================================================================

use freedesktop_entry_parser::parse_entry;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use std::collections::HashSet;
use std::env;
use std::fs;
use std::io;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use walkdir::WalkDir;

use crate::db::Database;

#[derive(Clone, Debug)]
pub struct AppEntry {
    pub name: String,
    pub exec: String,
    pub is_cli_only: bool,
    pub total_score: i32,
    pub is_dormant: bool,
}

// ============================================================================
// APP SCANNING WITH CACHE
// ============================================================================

/// Load apps from cache or rescan if cache is stale
pub fn scan_apps(db: &Database) -> Vec<AppEntry> {
    // Try to load from cache first
    if let Ok(cached) = load_app_cache() {
        if !cached.is_empty() {
            return enrich_apps_with_db_data(cached, db);
        }
    }

    // Cache miss - do full scan and rebuild cache
    let apps = scan_apps_fresh(db);
    let _ = save_app_cache(&apps);
    apps
}

/// Force rebuild the app cache
pub fn rebuild_app_cache(db: &Database) -> io::Result<()> {
    let apps = scan_apps_fresh(db);
    save_app_cache(&apps)?;
    Ok(())
}

fn scan_apps_fresh(db: &Database) -> Vec<AppEntry> {
    let mut apps = Vec::new();
    let mut seen_names = HashSet::new();
    let mut known_execs = HashSet::new();

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let thirty_days = 30 * 24 * 60 * 60;

    // OPTIMIZATION: Batch load all DB data in one query (eliminates N+1 problem)
    let db_data = db.get_all_app_data();

    // Scan .desktop files
    let dirs = [
        "/usr/share/applications",
        "/usr/local/share/applications",
        "/home/linuxbrew/.linuxbrew/share/applications",
    ];

    let home_apps = dirs::home_dir().map(|h| h.join(".local/share/applications"));
    let mut search_dirs: Vec<PathBuf> = dirs.iter().map(PathBuf::from).collect();
    if let Some(h) = home_apps {
        search_dirs.push(h);
    }

    for dir in search_dirs {
        if !dir.exists() {
            continue;
        }

        for entry in WalkDir::new(dir)
            .max_depth(1)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.path().extension().map_or(false, |e| e == "desktop") {
                if let Ok(entry_file) = parse_entry(entry.path()) {
                    // 1. Get the section safely. If missing, skip this file.
                    let section = match entry_file.section("Desktop Entry") {
                        Some(s) => s,
                        None => continue,
                    };

                    // 2. Handle NoDisplay (attr returns a list now, take the first item)
                    let no_display = section
                        .attr("NoDisplay")
                        .first() // Get Option<&String>
                        .map(|s| s == "true")
                        .unwrap_or(false);

                    if no_display {
                        continue;
                    }

                    // 3. Handle Name
                    let name = section
                        .attr("Name")
                        .first()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| "Unknown".to_string());

                    // 4. Handle Exec
                    let exec_raw = section
                        .attr("Exec")
                        .first()
                        .map(|s| s.to_string())
                        .unwrap_or_default();

                    if !exec_raw.is_empty() && name != "Unknown" {
                        let binary_name = exec_raw
                            .split_whitespace()
                            .next()
                            .unwrap_or("")
                            .to_string();

                        let simple_bin = Path::new(&binary_name)
                            .file_name()
                            .map(|s| s.to_string_lossy().to_string())
                            .unwrap_or(binary_name);

                        known_execs.insert(simple_bin);

                        if seen_names.insert(name.clone()) {
                            // Use batch-loaded DB data instead of individual query
                            let (_, base_score, usage, last_used) = db_data
                                .get(&name)
                                .copied()
                                .unwrap_or((false, 0, 0, 0));
                            
                            let total = base_score + (usage * 10);
                            let is_dormant =
                                last_used > 0 && (now.saturating_sub(last_used) > thirty_days);

                            apps.push(AppEntry {
                                name,
                                exec: exec_raw,
                                is_cli_only: false,
                                total_score: total,
                                is_dormant,
                            });
                        }
                    }
                }
            }
        }
    }

    // Scan $PATH executables
    if let Ok(path_var) = env::var("PATH") {
        for path_str in path_var.split(':') {
            let dir = PathBuf::from(path_str);
            if !dir.exists() || !dir.is_dir() {
                continue;
            }
            if path_str.contains("/sbin")
                || path_str.contains("/games")
                || path_str.contains("/lib")
            {
                continue;
            }

            if let Ok(entries) = fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() {
                        let name_str = path.file_name().unwrap().to_string_lossy();
                        if name_str.contains('.') || name_str.starts_with('.') {
                            continue;
                        }

                        if let Ok(metadata) = path.metadata() {
                            if metadata.permissions().mode() & 0o111 != 0 {
                                let name = name_str.to_string();

                                if known_execs.contains(&name) {
                                    continue;
                                }

                                if seen_names.insert(name.clone()) {
                                    // Use batch-loaded DB data instead of individual query
                                    let (_, base_score, usage, last_used) = db_data
                                        .get(&name)
                                        .copied()
                                        .unwrap_or((false, 0, 0, 0));
                                    
                                    let total = base_score + (usage * 10);
                                    let is_dormant = last_used > 0
                                        && (now.saturating_sub(last_used) > thirty_days);

                                    apps.push(AppEntry {
                                        name: name.clone(),
                                        exec: name,
                                        is_cli_only: true,
                                        total_score: total,
                                        is_dormant,
                                    });
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    apps.sort_by(|a, b| {
        b.total_score
            .cmp(&a.total_score)
            .then_with(|| a.name.cmp(&b.name))
    });

    apps
}

/// Enrich cached apps with fresh database data
fn enrich_apps_with_db_data(mut apps: Vec<AppEntry>, db: &Database) -> Vec<AppEntry> {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let thirty_days = 30 * 24 * 60 * 60;

    for app in &mut apps {
        let (_, base_score, usage, last_used) = db.get_app_data(&app.name);
        app.total_score = base_score + (usage * 10);
        app.is_dormant = last_used > 0 && (now.saturating_sub(last_used) > thirty_days);
    }

    apps.sort_by(|a, b| {
        b.total_score
            .cmp(&a.total_score)
            .then_with(|| a.name.cmp(&b.name))
    });

    apps
}

// ============================================================================
// APP CACHE PERSISTENCE
// ============================================================================

#[derive(serde::Serialize, serde::Deserialize)]
struct CachedApp {
    name: String,
    exec: String,
    is_cli_only: bool,
}

fn get_cache_path() -> PathBuf {
    let mut path = dirs::cache_dir().unwrap_or_else(|| PathBuf::from("."));
    path.push("rula");
    std::fs::create_dir_all(&path).ok();
    path.push("apps.json");
    path
}

fn save_app_cache(apps: &[AppEntry]) -> io::Result<()> {
    let cached: Vec<CachedApp> = apps
        .iter()
        .map(|a| CachedApp {
            name: a.name.clone(),
            exec: a.exec.clone(),
            is_cli_only: a.is_cli_only,
        })
        .collect();

    let json = serde_json::to_string(&cached)?;
    fs::write(get_cache_path(), json)?;
    Ok(())
}

fn load_app_cache() -> io::Result<Vec<AppEntry>> {
    let json = fs::read_to_string(get_cache_path())?;
    let cached: Vec<CachedApp> = serde_json::from_str(&json)?;

    let apps = cached
        .into_iter()
        .map(|c| AppEntry {
            name: c.name,
            exec: c.exec,
            is_cli_only: c.is_cli_only,
            total_score: 0,
            is_dormant: false,
        })
        .collect();

    Ok(apps)
}

// ============================================================================
// FILE STREAMING SEARCH (fd-like performance)
// ============================================================================

pub struct FileSearcher {
    home: PathBuf,
}

impl FileSearcher {
    pub fn new() -> Self {
        Self {
            home: dirs::home_dir().unwrap_or_else(|| PathBuf::from("/")),
        }
    }

    /// Stream file search - returns results as they're found (lazy)
    /// OPTIMIZED: Uses rayon for parallel fuzzy matching
    pub fn search(&self, query: &str, limit: usize) -> Vec<String> {
        use rayon::prelude::*;
        
        if query.is_empty() {
            return Vec::new();
        }

        let query_lower = query.to_lowercase();

        // Step 1: Collect candidate paths (with pre-filter)
        let mut candidates = Vec::new();
        let walker = ignore::WalkBuilder::new(&self.home)
            .hidden(false)
            .max_depth(Some(5))
            .git_ignore(true)
            .ignore(true)
            .build();

        for entry in walker {
            // Collect more candidates for better fuzzy matching
            if candidates.len() >= limit * 10 {
                break;
            }

            if let Ok(entry) = entry {
                if entry.file_type().map_or(false, |ft| ft.is_file()) {
                    let path_str = entry.path().to_string_lossy().to_string();

                    // Quick pre-filter: skip if doesn't contain query chars
                    let path_lower = path_str.to_lowercase();
                    if query_lower.chars().all(|c| path_lower.contains(c)) {
                        candidates.push(path_str);
                    }
                }
            }
        }

        // Step 2: PARALLEL fuzzy matching with rayon
        let matcher = SkimMatcherV2::default();
        let mut results: Vec<(i64, String)> = candidates
            .par_iter()  // <-- RAYON: Parallel iterator
            .filter_map(|path| {
                matcher.fuzzy_match(path, query).map(|score| (score, path.clone()))
            })
            .collect();

        // Step 3: Sort and return top N
        results.sort_by(|a, b| b.0.cmp(&a.0));
        results.truncate(limit);
        results.into_iter().map(|(_, path)| path).collect()
    }
}

// ============================================================================
// FUZZY SEARCH FOR APPS
// ============================================================================

pub fn fuzzy_search_apps<'a>(query: &str, apps: &'a [AppEntry]) -> Vec<&'a AppEntry> {
    use rayon::prelude::*;
    
    let matcher = SkimMatcherV2::default();
    
    // RAYON: Parallel fuzzy matching for apps
    let mut matches: Vec<_> = apps
        .par_iter()  // <-- PARALLEL
        .filter_map(|app| matcher.fuzzy_match(&app.name, query).map(|s| (s, app)))
        .collect();

    matches.sort_by(|a, b| {
        b.0.cmp(&a.0)
            .then(b.1.total_score.cmp(&a.1.total_score))
    });

    matches.into_iter().take(50).map(|(_, i)| i).collect()
}

// ============================================================================
// DATABASE SEEDING
// ============================================================================

pub fn seed_database(db: &Database) {
    println!("Seeding database from Pacman... this might take a few seconds.");

    let output = Command::new("sh")
        .arg("-c")
        .arg("pacman -Qqe | xargs pacman -Ql | grep '/usr/bin/'")
        .output()
        .expect("Failed to execute pacman");

    let stdout = String::from_utf8_lossy(&output.stdout);

    let mut count = 0;
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() >= 2 {
            let path = Path::new(parts[1]);
            if let Some(name_os) = path.file_name() {
                let name = name_os.to_string_lossy().to_string();
                let _ = db.set_base_score(&name, 50);
                count += 1;
            }
        }
    }

    println!("Seeded {} apps with +50 score.", count);
}
