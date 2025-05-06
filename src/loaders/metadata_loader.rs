use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use url::Url;

use crate::utils::misc::adjust_file_path;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Metadata {
    pub name: String,
    pub symbol: String,
    pub description: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub twitter: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub telegram: String,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub website: String,
    #[serde(skip)] // This field will be ignored during JSON parsing
    pub show_name: bool,
}

fn is_valid_url(url: &str) -> bool {
    if url.is_empty() {
        return true;
    }
    Url::parse(url).is_ok()
}

pub fn validate_and_retrieve_metadata(use_video: bool) -> Result<(Metadata, String), String> {
    // Check if the media folder exists
    let file_path = &adjust_file_path("configurations/pump/media");
    let media_path = Path::new(file_path);
    if !media_path.exists() {
        return Err("Media folder is missing.".to_string());
    }

    // Get list of files in the media folder
    let files = fs::read_dir(media_path)
        .map_err(|_| "Unable to read media folder.".to_string())?
        .filter_map(Result::ok)
        .map(|entry| entry.file_name().to_string_lossy().to_string())
        .collect::<Vec<String>>();

    // Filter image files
    let image_files: Vec<String> = files
        .iter()
        .filter(|file| {
            let lower_case = file.to_lowercase();
            lower_case.starts_with("image.")
                && (lower_case.ends_with(".png")
                    || lower_case.ends_with(".jpeg")
                    || lower_case.ends_with(".jpg")
                    || lower_case.ends_with(".gif"))
        })
        .cloned()
        .collect();

    if image_files.is_empty() {
        return Err(
            "'image' file not found in media folder. Allowed formats: .png, .jpeg, .jpg, .gif"
                .to_string(),
        );
    }

    if image_files.len() > 1 {
        return Err("Cannot have more than 1 'image' file in the media folder".to_string());
    }

    let image_file = &image_files[0];
    let image_path = media_path.join(image_file);
    if !image_path.exists() {
        return Err(format!(
            "Could not find image file {} in media folder",
            image_file
        ));
    }

    if use_video {
        // Check for video file (video.mp4)
        let video_file_exists = files.iter().any(|file| {
            let lower_case = file.to_lowercase();
            lower_case == "video.mp4"
        });
        if !video_file_exists {
            return Err("Video file 'video.mp4' not found in media folder.".to_string());
        }
    }

    // Check metadata.json
    let file_path = &adjust_file_path("configurations/pump/metadata.json"); 
    let metadata_path = Path::new(file_path);
    //let metadata_path = media_path.join("metadata.json");
    if !metadata_path.exists() {
        return Err("No metadata.json file found in media folder".to_string());
    }

    // Read and parse JSON
    let raw_data = fs::read_to_string(&metadata_path)
        .map_err(|_| "Error reading metadata.json file".to_string())?;

    let mut metadata: Metadata = serde_json::from_str(&raw_data)
        .map_err(|e| format!("Error parsing the file content: {}.", e))?;

    metadata.show_name = true;

    // Validate fields
    if metadata.name.is_empty() {
        return Err("Metadata field 'name' cannot be empty.".to_string());
    }

    if metadata.symbol.is_empty() {
        return Err("Metadata field 'symbol' cannot be empty.".to_string());
    }

    if metadata.description.is_empty() {
        return Err("Metadata field 'description' cannot be empty.".to_string());
    }

    if metadata.name.len() > 32 {
        return Err("Name too long, can't exceed 32 characters.".to_string());
    }

    if metadata.symbol.len() > 11 {
        return Err("Symbol too long, can't exceed 11 characters.".to_string());
    }

    if !is_valid_url(&metadata.website) {
        return Err("Invalid website URL.".to_string());
    }

    // Set the image file path in the metadata
    //metadata.image_file_path = image_file.clone();

    Ok((metadata, image_file.clone()))
}
