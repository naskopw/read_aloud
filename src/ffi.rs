use std::{
    cell::RefCell,
    ffi::{c_char, CStr, CString},
    path::Path,
};

use crate::{TTSError, Voice};

thread_local! {
    static LAST_ERROR_MESSAGE: RefCell<Option<CString>> = RefCell::new(None);
}

/// Enum representing the possible errors that can occur during text-to-speech generation.
#[repr(C)]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReadAloudStatus {
    /// The operation was successful.
    Success = 0,
    /// One or more inputs were invalid.
    InvalidInput = 1,
    /// The library could not connect to the speech service.
    ConnectionFailed = 2,
    /// The speech service returned an unexpected response.
    ProtocolError = 3,
    /// A local I/O operation failed.
    IoError = 4,
    /// An unexpected internal failure occurred.
    InternalError = 255,
}

impl From<TTSError> for ReadAloudStatus {
    fn from(e: TTSError) -> Self {
        match e {
            TTSError::InvalidInput(_) | TTSError::NullPointer(_) | TTSError::Utf8(_) => {
                ReadAloudStatus::InvalidInput
            }
            TTSError::Connection(_) => ReadAloudStatus::ConnectionFailed,
            TTSError::Protocol(_) => ReadAloudStatus::ProtocolError,
            TTSError::Io(_) => ReadAloudStatus::IoError,
            TTSError::Internal(_) => ReadAloudStatus::InternalError,
        }
    }
}

fn clear_last_error() {
    LAST_ERROR_MESSAGE.with(|slot| {
        *slot.borrow_mut() = None;
    });
}

fn set_last_error(message: &str) {
    let sanitized = message.replace('\0', " ");
    let c_message = match CString::new(sanitized) {
        Ok(message) => message,
        Err(_) => return,
    };
    LAST_ERROR_MESSAGE.with(|slot| {
        *slot.borrow_mut() = Some(c_message);
    });
}

fn record_error(error: TTSError) -> ReadAloudStatus {
    set_last_error(&error.to_string());
    error.into()
}

fn static_status_message(status: ReadAloudStatus) -> *const c_char {
    match status {
        ReadAloudStatus::Success => b"success\0".as_ptr().cast(),
        ReadAloudStatus::InvalidInput => b"invalid input\0".as_ptr().cast(),
        ReadAloudStatus::ConnectionFailed => b"connection failed\0".as_ptr().cast(),
        ReadAloudStatus::ProtocolError => b"protocol error\0".as_ptr().cast(),
        ReadAloudStatus::IoError => b"I/O error\0".as_ptr().cast(),
        ReadAloudStatus::InternalError => b"internal error\0".as_ptr().cast(),
    }
}

fn parse_c_string<'a>(ptr: *const c_char, field: &'static str) -> Result<&'a str, TTSError> {
    if ptr.is_null() {
        return Err(TTSError::NullPointer(field.into()));
    }

    let value = unsafe { CStr::from_ptr(ptr) };
    value
        .to_str()
        .map_err(|_| TTSError::Utf8(field.into()))
}

#[no_mangle]
pub extern "C" fn read_aloud_status_string(status: ReadAloudStatus) -> *const c_char {
    static_status_message(status)
}

#[no_mangle]
pub extern "C" fn read_aloud_last_error_message() -> *const c_char {
    LAST_ERROR_MESSAGE.with(|slot| {
        let slot = slot.borrow();
        slot.as_ref()
            .map(|message| message.as_ptr())
            .unwrap_or_else(|| b"\0".as_ptr().cast())
    })
}

#[no_mangle]
pub extern "C" fn text_to_speech(
    text: *const c_char,
    voice: Voice,
    pitch: i32,
    rate: f32,
    volume: f32,
    f: *const c_char,
) -> ReadAloudStatus {
    clear_last_error();

    let result = std::panic::catch_unwind(|| {
        let text = parse_c_string(text, "text")?;
        let f = parse_c_string(f, "output path")?;
        super::generate(text, voice, pitch, rate, volume, Path::new(f))
    });

    match result {
        Ok(Ok(())) => ReadAloudStatus::Success,
        Ok(Err(error)) => record_error(error),
        Err(_) => {
            set_last_error("panic in text_to_speech");
            ReadAloudStatus::InternalError
        }
    }
}
