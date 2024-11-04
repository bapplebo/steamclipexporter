use std::path::{Path, PathBuf};

pub fn sort_chunks(chunk_files: &mut Vec<PathBuf>) {
    chunk_files.sort_by(|a, b| {
        // Extract the numeric part from the file names for comparison
        let a_num = a
            .file_name()
            .and_then(|s| s.to_str())
            .and_then(|s| s.split('-').last())
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        let b_num = b
            .file_name()
            .and_then(|s| s.to_str())
            .and_then(|s| s.split('-').last())
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);

        a_num.cmp(&b_num)
    });
}

pub fn parse_clip_string(clip_string: &str) -> (u64, u64, u64) {
    let path = Path::new(clip_string);
    let last_part = path.file_name().unwrap().to_str().unwrap();
    let trimmed_part = last_part.trim_start_matches("clip_");
    let parts: Vec<&str> = trimmed_part.split('_').collect();
    println!("parts: {:?}", parts);
    let clip_number = parts[0].parse().unwrap();
    let date = parts[1].parse().unwrap();
    let time = parts[2].parse().unwrap();

    (clip_number, date, time)
}
