use clap::Parser;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{fs, io};

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

const INIT_VIDEO_FILE: &str = "init-stream0.m4s";
const INIT_AUDIO_FILE: &str = "init-stream1.m4s";

fn main() {
    let args = Args::parse();

    let directory = args.directory;
    println!("directory {}", directory);

    let directory_path = Path::new(directory.as_str());
    let subdirectories = get_subdirectories(directory_path);

    // Take the top for now to test processing
    match subdirectories {
        Ok(subdirectories) => {
            let first = subdirectories[0].clone();
            println!("Subdirectory: {}", first);
            let video_clips_directory = validate_clip_directory(first.as_str())
                .map(|res| res.unwrap_or_default())
                .unwrap_or_default();

            println!("Clips directory: {}", video_clips_directory);

            concat_m4s_files(Path::new(video_clips_directory.as_str()), "");
        }
        Err(error) => {
            println!("Error fetching subdirectories for {}: {}", directory, error)
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

// C:\steamrecordings\clips\clip_238960_20240815_015514\video\bg_238960_20240815_014523
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

fn concat_m4s_files(dir: &Path, output_file: &str) -> io::Result<()> {
    println!("Starting concat...");
    let init_video_file_path = dir.join(INIT_VIDEO_FILE);
    let init_audio_file_path = dir.join(INIT_AUDIO_FILE);

    if init_video_file_path.exists() && init_audio_file_path.exists() {
        // Process video
        concat_video_files(init_video_file_path, dir);
        concat_audio_files(init_audio_file_path, dir);
        Ok(())
    } else {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "Init files not found, unable to process clip",
        ));
    }
}

fn concat_video_files(init_video_file_path: PathBuf, dir: &Path) -> io::Result<()> {
    println!("Processing video...");
    let mut command = get_command();
    command.arg(init_video_file_path);

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file()
            && path
                .file_name()
                .and_then(|s| s.to_str())
                .map_or(false, |s| s.starts_with("chunk-stream0"))
        {
            command.arg(path);
        }
    }

    // More windows specific things
    if cfg!(target_os = "windows") {
        command.arg("+");
    }

    command.arg("./output_file.mp4");

    let x = format!("{:?}", command);
    println!("{}", x);

    // Execute our command
    let output = command.output()?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Failed to concatenate files: {}",
                String::from_utf8_lossy(&output.stderr)
            ),
        ));
    }

    println!("Finished concatting video files...");
    let x = format!("{:?}", command);
    println!("{}", x);

    Ok(())
}

fn concat_audio_files(init_audio_file_path: PathBuf, dir: &Path) {
    let mut command = get_command();
}

fn get_command() -> Command {
    let command = if cfg!(target_os = "windows") {
        let mut win_command = Command::new("cmd");
        win_command.arg("/C").arg("copy").arg("/b");
        win_command
    } else {
        let unix_command = Command::new("cat");
        unix_command
    };

    command
}
