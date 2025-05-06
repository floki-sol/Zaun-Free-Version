use chrono::{DateTime, Local, NaiveDateTime, TimeZone};
use std::cmp::Reverse;
use std::fs::{self, File, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use crate::constants::general::LookupTableFileFormat;

use super::misc::adjust_file_path;

const MAX_BACKUPS_BUNDLE_WALLETS: usize = 20;
const MAX_BACKUPS_LOOKUP_TABLES: usize = 100;
const MAX_BACKUPS_LAUNCH_MANIFEST: usize = 1000;
const BACKUP_DIR_BUNDLE_WALLET: &str = "temp/backups/bundler-wallets";
const BACKUP_DIR_LOOKUP_TABLES: &str = "temp/backups/lookup-tables";
const BACKUP_DIR_LAUNCH_MANIFESTS: &str = "temp/backups/launch_manifests";
pub enum BackupType {
    BundleWallets,
    LookupTables((String, String)),
    LaunchManifest,
}

/// Creates a backup of the existing bundler wallets file and maintains a rotating buffer of backups
pub fn backup_files(backup_type: BackupType) -> Result<(), String> {
    let backup_dir: &str = match backup_type {
        BackupType::BundleWallets => BACKUP_DIR_BUNDLE_WALLET,
        BackupType::LookupTables((_, _)) => BACKUP_DIR_LOOKUP_TABLES,
        BackupType::LaunchManifest => BACKUP_DIR_LAUNCH_MANIFESTS,
    };

    // Create backup directory if it doesn't exist
    fs::create_dir_all(&adjust_file_path(backup_dir)).map_err(|e| format!("{e}"))?;

    match backup_type {
        BackupType::BundleWallets => {
            let bundler_file: &str = &adjust_file_path("configurations/pump/bundler-wallets.json");

            // Check if original file exists before attempting backup
            if Path::new(bundler_file).exists() {
                // Read existing content
                let mut content = String::new();
                File::open(bundler_file).map_err(|e| format!("{e}"))?.read_to_string(&mut content).map_err(|e| format!("{e}"))?;

                // Generate backup filename with timestamp
                let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
                let backup_path = &adjust_file_path(&format!("{}/{}.json", backup_dir, timestamp));

                // Write backup
                let mut backup_file = File::create(&backup_path).map_err(|e| format!("{e}"))?;
                backup_file.write_all(content.as_bytes()).map_err(|e| format!("{e}"))?;
            }
        }

        BackupType::LookupTables((ref lookup_table_address, ref mint)) => {
            let content: LookupTableFileFormat = LookupTableFileFormat {
                lookup_table: lookup_table_address.clone(),
                mint: mint.clone(),
            };
            let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
            let backup_path = &adjust_file_path(&format!("{}/{}.json", backup_dir, timestamp));

            // Write backup
            let mut backup_file = File::create(&backup_path).map_err(|e| format!("{e}"))?;
            backup_file.write_all(serde_json::to_string_pretty(&content).unwrap().as_bytes()).map_err(|e| format!("{e}"))?;
        }

        BackupType::LaunchManifest => {
            let launch_manifest_file: &str =
                &adjust_file_path("configurations/pump/launch-manifest.json");

            // Check if original file exists before attempting backup
            if Path::new(launch_manifest_file).exists() {
                // Read existing content
                let mut content = String::new();
                File::open(launch_manifest_file).map_err(|e| format!("{e}"))?.read_to_string(&mut content).map_err(|e| format!("{e}"))?;

                // Generate backup filename with timestamp
                let timestamp = Local::now().format("%Y-%m-%d_%H-%M-%S");
                let backup_path = &adjust_file_path(&format!("{}/{}.json", backup_dir, timestamp));

                // Write backup
                let mut backup_file = File::create(&backup_path).map_err(|e| format!("{e}"))?;
                backup_file.write_all(content.as_bytes()).map_err(|e| format!("{e}"))?;
            }
        }
    };

    // Manage backup rotation
    cleanup_old_backups(backup_type)?;

    Ok(())
}

/// Removes oldest backups if total exceeds MAX_BACKUPS
pub fn cleanup_old_backups(backup_type: BackupType) -> Result<(), String> {
    let backup_dir: &str = match backup_type {
        BackupType::BundleWallets => BACKUP_DIR_BUNDLE_WALLET,
        BackupType::LaunchManifest => BACKUP_DIR_LAUNCH_MANIFESTS,
        BackupType::LookupTables((_, _)) => BACKUP_DIR_LOOKUP_TABLES,
    };

    // Get all backup files
    let mut backups: Vec<PathBuf> = fs::read_dir(&adjust_file_path(backup_dir)).map_err(|e| format!("{e}"))?
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect();

    // Sort by modification time (oldest first)
    backups.sort_by(|a, b| {
        fs::metadata(a)
            .and_then(|meta| meta.modified())
            .unwrap_or_else(|_| std::time::SystemTime::UNIX_EPOCH)
            .cmp(
                &fs::metadata(b)
                    .and_then(|meta| meta.modified())
                    .unwrap_or_else(|_| std::time::SystemTime::UNIX_EPOCH),
            )
    });

    let max_backups = match backup_type {
        BackupType::BundleWallets => MAX_BACKUPS_BUNDLE_WALLETS,
        BackupType::LaunchManifest => MAX_BACKUPS_LAUNCH_MANIFEST,
        BackupType::LookupTables(_) => MAX_BACKUPS_LOOKUP_TABLES,
    };

    // Remove oldest files if we exceed MAX_BACKUPS
    while backups.len() > max_backups {
        if let Some(oldest) = backups.first() {
            fs::remove_file(oldest).map_err(|e| format!("{e}"))?;
            backups.remove(0);
        }
    }

    Ok(())
}

pub fn load_most_recent_lut() -> Result<LookupTableFileFormat, String> {
    let backup_dir = BACKUP_DIR_LOOKUP_TABLES;

    let _ = fs::create_dir_all(&adjust_file_path(backup_dir));

    // Read directory contents
    let mut entries = (fs::read_dir(&adjust_file_path(backup_dir)))
        .map_err(|e| format!("Failed to read backup directory: {}", e))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect::<Vec<_>>();

    entries.sort_by(|a, b| {
        let a_time = a
            .metadata()
            .ok()
            .and_then(|meta| meta.modified().ok())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        let b_time = b
            .metadata()
            .ok()
            .and_then(|meta| meta.modified().ok())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        b_time.cmp(&a_time)
    });

    // Get the most recent entry
    let most_recent_file = entries
        .first()
        .ok_or_else(|| "No lookup table file found".to_string())?;

    // Read the file contents
    let file_path = most_recent_file.path();
    let content = fs::read_to_string(&file_path)
        .map_err(|e| format!("Failed to read lookup table file: {}", e))?;

    // Deserialize the JSON
    serde_json::from_str(&content).map_err(|e| format!("Failed to parse file: {}", e))
}

pub fn remove_most_recent_lut() -> Result<(), String> {
    let backup_dir = BACKUP_DIR_LOOKUP_TABLES;

    let _ = fs::create_dir_all(&adjust_file_path(backup_dir));

    // Read directory contents
    let mut entries = fs::read_dir(&adjust_file_path(backup_dir))
        .map_err(|e| format!("Failed to read backup directory: {}", e))?
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
        .collect::<Vec<_>>();

    entries.sort_by(|a, b| {
        let a_time = a
            .metadata()
            .ok()
            .and_then(|meta| meta.modified().ok())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        let b_time = b
            .metadata()
            .ok()
            .and_then(|meta| meta.modified().ok())
            .unwrap_or(SystemTime::UNIX_EPOCH);

        b_time.cmp(&a_time)
    });

    // Get the most recent entry
    let most_recent_file = entries
        .first()
        .ok_or_else(|| "No lookup table file found".to_string())?;

    // Remove the file
    let file_path = most_recent_file.path();
    fs::remove_file(&file_path).map_err(|e| format!("Failed to remove lookup table file: {}", e))
}

/// Retrieves all backup files for bundle wallets
pub fn get_bundle_wallet_backups() -> Result<Vec<PathBuf>, String> {
    let backup_dir = BACKUP_DIR_BUNDLE_WALLET;
    let _ = fs::create_dir_all(&adjust_file_path(backup_dir));
    let mut backups: Vec<PathBuf> = match fs::read_dir(&adjust_file_path(backup_dir)) {
        Ok(entries) => entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("json"))
            .collect(),
        Err(e) => return Err(format!("Failed to read backup directory: {}", e)),
    };

    // Sort by modification time, newest first
    backups.sort_by_key(|path| {
        Reverse(
            fs::metadata(path)
                .and_then(|meta| meta.modified())
                .unwrap_or_else(|_| std::time::SystemTime::UNIX_EPOCH),
        )
    });

    Ok(backups)
}

pub fn format_backup_filename(path: &Path) -> String {
    // Parse the filename timestamp
    if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
        if let Ok(naive_datetime) = NaiveDateTime::parse_from_str(stem, "%Y-%m-%d_%H:%M:%S") {
            let datetime: DateTime<Local> = Local.from_local_datetime(&naive_datetime).unwrap();

            // Get current time for comparison
            let now = Local::now();

            // Format based on how recent the backup is
            if now.date_naive() == datetime.date_naive() {
                // Today: Show time
                format!("Backup from today at {}", datetime.format("%I:%M %p"))
            } else if now
                .date_naive()
                .signed_duration_since(datetime.date_naive())
                .num_days()
                < 7
            {
                // Within last week: Show day of week and time
                format!(
                    "Backup from {} at {}",
                    datetime.format("%A"),
                    datetime.format("%I:%M %p")
                )
            } else {
                // Older: Show full date
                format!("Backup on {}", datetime.format("%B %d, %Y at %I:%M %p"))
            }
        } else {
            // Fallback if parsing fails
            stem.to_string()
        }
    } else {
        "Unknown Backup".to_string()
    }
}

pub fn restore_bundle_wallet_backup(backup_path: &Path) -> Result<(), String> {
    let bundler_wallets_path: &str = &adjust_file_path("configurations/pump/bundler-wallets.json");

    // Backup current bundler-wallets.json first
    backup_files(BackupType::BundleWallets)
        .map_err(|e| format!("Failed to create pre-restore backup: {}", e))?;

    // Read backup file contents
    let backup_contents = fs::read_to_string(backup_path)
        .map_err(|e| format!("Failed to read backup file: {}", e))?;

    // Validate JSON content
    let _validated_json: Vec<String> = serde_json::from_str(&backup_contents)
        .map_err(|e| format!("Invalid JSON in backup file: {}", e))?;

    // Write backup contents to file
    fs::write(bundler_wallets_path, &backup_contents)
        .map_err(|e| format!("Failed to restore backup: {}", e))?;

    // Remove the original backup file
    fs::remove_file(backup_path)
        .map_err(|e| format!("Failed to delete restored backup file: {}", e))?;

    Ok(())
}
