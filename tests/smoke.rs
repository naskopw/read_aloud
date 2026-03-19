use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use read_aloud::{text_to_speech, SpeechOptions, Voice};

#[test]
fn smoke_text_to_speech_generates_audio() {
    let output_path = smoke_output_path();
    let result = text_to_speech(
        "Hello, World!",
        Voice::en_GB_ThomasNeural,
        SpeechOptions::default(),
        &output_path,
    );

    match result {
        Ok(()) => {
            let metadata = fs::metadata(&output_path).expect("smoke test output should exist");
            assert!(metadata.len() > 0, "smoke test output should not be empty");
            let _ = fs::remove_file(&output_path);
        }
        Err(error) => {
            let _ = fs::remove_file(&output_path);
            panic!("smoke test failed: {error}");
        }
    }
}

fn smoke_output_path() -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after Unix epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("read_aloud_smoke_{unique}.mp3"))
}