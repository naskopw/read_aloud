use std::{
    ffi::{c_char, c_int, CStr},
    path::PathBuf,
};

use crate::Voice;

#[no_mangle]
pub extern "C" fn text_to_speech(text: *const c_char, voice: Voice, f: *const c_char) -> c_int {
    let text = unsafe { CStr::from_ptr(text) };
    let text = text.to_str().unwrap();
    let f = unsafe { CStr::from_ptr(f) };
    let f = f.to_str().unwrap();
    match super::generate(text, voice, PathBuf::from(f)) {
        Ok(_) => 0,
        Err(_) => 1,
    }
}
