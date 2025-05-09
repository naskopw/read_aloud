use std::{
    ffi::{c_char, CStr},
    path::Path,
};

use crate::{TTSError, Voice};

/// Enum representing the possible errors that can occur during text-to-speech generation.
#[repr(C)]
pub enum TextToSpeechError {
    /// The operation was successful.
    Success = 0,
    /// The generation of the audio file failed.
    GenerationFailed = 1,
}

impl From<TTSError> for TextToSpeechError {
    fn from(e: TTSError) -> Self {
        match e {
            _ => TextToSpeechError::GenerationFailed,
        }
    }
}

#[no_mangle]
pub extern "C" fn text_to_speech(
    text: *const c_char,
    voice: Voice,
    pitch: i32,
    rate: f32,
    volume: f32,
    f: *const c_char,
) -> TextToSpeechError {
    let text = unsafe { CStr::from_ptr(text) };
    let text = text.to_str().unwrap();
    let f = unsafe { CStr::from_ptr(f) };
    let f = f.to_str().unwrap();
    match super::generate(text, voice, pitch, rate, volume, Path::new(f)) {
        Ok(_) => TextToSpeechError::Success,
        Err(e) => e.into(),
    }
}
