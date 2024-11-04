use clap::Parser;
use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use tempfile::TempDir;

mod utils;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Directory containing steam clips
    #[arg(short, long, value_parser = validate_directory)]
    directory: String,

    /// Directory where exported clips will end up. By default will be located in the directory passed into the directory argument.
    #[arg(short, long, value_parser = validate_directory)]
    output: Option<String>,

    /// Verbose mode
    #[arg(short, long, default_value_t = false)]
    verbose: bool,
}

#[derive(Serialize, Deserialize, Debug)]
struct AppDetails {
    #[serde(flatten)]
    properties: HashMap<String, serde_json::Value>,
}

const INIT_VIDEO_FILE: &str = "init-stream0.m4s";
const INIT_AUDIO_FILE: &str = "init-stream1.m4s";

fn main() {
    let args = Args::parse();
    let directory_path = Path::new(args.directory.as_str());

    // Take the top for now to test processing
    match get_subdirectories(directory_path) {
        Ok(subdirectories) => {
            println!("{:?}", subdirectories);
            let first = subdirectories[6].clone(); // <- iterate through entire folder from here
            println!("Subdirectory: {}", first);

            let (steam_id, date, time) = utils::parse_clip_string(first.as_str());

            let game_name = match get_app_details(steam_id) {
                Ok(app_details) => app_details
                    .properties
                    .get(&steam_id.to_string())
                    .and_then(|game_details| game_details.get("data"))
                    .and_then(|game_data| game_data.get("name"))
                    .and_then(|name| Some(sanitize_filename::sanitize(name.to_string())))
                    .unwrap_or_else(|| {
                        println!("Error fetching app details for: {}", steam_id);
                        "clip".to_string()
                    }),
                Err(error) => {
                    println!("Error fetching app details for: {}, {}", steam_id, error);
                    "clip".to_string()
                }
            };

            let video_clips_directory = validate_clip_directory(first.as_str())
                .map(|res| res.unwrap_or_default())
                .unwrap_or_default();

            println!("Clips directory: {}", video_clips_directory);
            let output_file_name = format!("{} {} {}", game_name, date, time);
            let _ = concat_m4s_files(Path::new(video_clips_directory.as_str()), output_file_name);
        }
        Err(error) => {
            println!(
                "Error fetching subdirectories for {}: {}",
                args.directory, error
            )
        }
    }
}

fn validate_directory(path: &str) -> Result<String, String> {
    if Path::new(path).is_dir() {
        Ok(path.to_string())
    } else {
        Err(format!("'{}' is not a valid directory path", path))
    }
}

fn validate_clip_directory(clip_path_str: &str) -> io::Result<Option<String>> {
    let clip_path = Path::new(clip_path_str);
    let video_dir = clip_path.join("video");
    if video_dir.is_dir() {
        for entry in fs::read_dir(video_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.is_dir()
                && path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map_or(false, |s| s.starts_with("bg_"))
            {
                return Ok(Some(path.to_string_lossy().to_string()));
            }
        }
    }
    Ok(None)
}

fn get_subdirectories(clips_directory: &Path) -> io::Result<Vec<String>> {
    let mut subdirectories = Vec::new();
    for entry in fs::read_dir(clips_directory)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            subdirectories.push(path.to_string_lossy().to_string());
        }
    }

    Ok(subdirectories)
}

fn concat_m4s_files(dir: &Path, output_file_name: String) -> io::Result<()> {
    println!("Starting concat...");
    let init_video_file_path = dir.join(INIT_VIDEO_FILE);
    let init_audio_file_path = dir.join(INIT_AUDIO_FILE);

    if init_video_file_path.exists() && init_audio_file_path.exists() {
        let tmp_dir = tempfile::tempdir().expect("Failed to create temporary directory");
        println!("Creating temp directory in: {:?}", tmp_dir.path());

        concat_video_files(init_video_file_path, dir, &tmp_dir)?;
        concat_audio_files(init_audio_file_path, dir, &tmp_dir)?;
        join_video_audio(&tmp_dir, output_file_name)?;

        cleanup(&tmp_dir);

        Ok(())
    } else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Init files not found, unable to process clip",
        ));
    }
}

fn concat_video_files(
    init_video_file_path: PathBuf,
    dir: &Path,
    tmp_dir: &TempDir,
) -> io::Result<()> {
    println!("Processing video...");

    let mut output_file = File::create(tmp_dir.path().join("tmp_video.mp4"))?;
    let mut init_file = File::open(init_video_file_path)?;

    io::copy(&mut init_file, &mut output_file);

    // Collect and sort chunk files
    let mut chunk_files: Vec<_> = fs::read_dir(dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_file()
                && path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map_or(false, |s| s.starts_with("chunk-stream0-"))
            {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    utils::sort_chunks(&mut chunk_files);

    // Append sorted chunk files
    for path in chunk_files {
        let mut chunk_file = File::open(path)?;
        io::copy(&mut chunk_file, &mut output_file)?;
    }

    println!("Finished concatting video files...");

    Ok(())
}

fn concat_audio_files(
    init_audio_file_path: PathBuf,
    dir: &Path,
    tmp_dir: &TempDir,
) -> io::Result<()> {
    println!("Processing audio...");

    let mut output_file = File::create(tmp_dir.path().join("tmp_audio.mp4"))?;
    let mut init_file = File::open(init_audio_file_path)?;

    io::copy(&mut init_file, &mut output_file);

    // Collect and sort chunk files
    let mut chunk_files: Vec<_> = fs::read_dir(dir)?
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            if path.is_file()
                && path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .map_or(false, |s| s.starts_with("chunk-stream1-"))
            {
                Some(path)
            } else {
                None
            }
        })
        .collect();

    utils::sort_chunks(&mut chunk_files);

    // Append sorted chunk files
    for path in chunk_files {
        let mut chunk_file = File::open(path)?;
        io::copy(&mut chunk_file, &mut output_file)?;
    }

    println!("Finished concatting audio files...");

    Ok(())
}

fn join_video_audio(tmp_dir: &TempDir, output_file_name: String) -> io::Result<()> {
    println!("Merging using ffmpeg...");

    let mut command = Command::new("ffmpeg")
        .arg("-i")
        .arg(tmp_dir.path().join("tmp_video.mp4"))
        .arg("-i")
        .arg(tmp_dir.path().join("tmp_audio.mp4"))
        .arg("-c")
        .arg("copy")
        // Extra commands to experiment with later
        // .arg("-c:v")
        // .arg("libx265")
        // .arg("-vtag")
        // .arg("hvc1")
        // .arg("-c:a")
        // .arg("copy")
        // .arg("-crf")
        // .arg("18")
        .arg(format!("{}.mp4", output_file_name))
        .stdout(Stdio::piped())
        .spawn()?;

    let output = command.stdout.take().unwrap();

    let mut buf_reader = io::BufReader::new(output);
    io::copy(&mut buf_reader, &mut io::stdout())?;

    let status = command.wait()?;

    if !status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to combine video and audio: {}", status),
        ));
    }

    Ok(())
}

// https://y.tsutsumi.io/reading-steam-game-recordings
// However seems to have issues - concatting each file works better for me
fn quick_join_video_audio(path: &Path) -> io::Result<()> {
    println!("Merging quickly using ffmpeg...");

    let mut command = Command::new("ffmpeg")
        .arg("-i")
        .arg(path.join("session.mpd"))
        .arg("-c")
        .arg("copy")
        .arg("output.mp4")
        .stdout(Stdio::piped())
        .spawn()?;

    let output = command.stdout.take().unwrap();

    let mut buf_reader = io::BufReader::new(output);
    io::copy(&mut buf_reader, &mut io::stdout())?;

    let status = command.wait()?;

    if !status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!("Failed to combine video and audio: {}", status),
        ));
    }

    Ok(())
}

fn cleanup(tmp_dir: &TempDir) {
    fs::remove_file(tmp_dir.path().join("tmp_video.mp4"));
    fs::remove_file(tmp_dir.path().join("tmp_audio.mp4"));
}

fn get_app_details(steam_id: u64) -> Result<AppDetails, reqwest::Error> {
    println!("Fetching app details from Steam API...");

    let client = Client::new();
    let url = format!(
        "https://store.steampowered.com/api/appdetails?appids={}",
        steam_id
    );
    let response = client.get(&url).send()?;
    response.error_for_status_ref()?;

    let app_details = response.json()?;
    Ok(app_details)
}
