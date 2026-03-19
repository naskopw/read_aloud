use std::{
    cell::RefCell,
    ffi::{c_char, CStr, CString},
    mem,
    ptr,
    path::Path,
};

use crate::{SpeechOptions, TTSError, Voice};

thread_local! {
    static LAST_ERROR_MESSAGE: RefCell<Option<CString>> = RefCell::new(None);
}

/// Numeric status codes returned by the C ABI.
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

/// Size-prefixed speech options for the C ABI.
///
/// Initialize this struct with [read_aloud_speech_options_init] before setting any fields.
/// Passing a null pointer for the `options` parameter of [read_aloud_text_to_speech] uses the
/// library defaults instead.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ReadAloudSpeechOptions {
    /// Size of the caller-provided struct in bytes.
    ///
    /// This lets the ABI accept prefix-compatible structs from older or newer callers.
    pub size: u32,
    /// Voice pitch adjustment in hertz.
    pub pitch_hz: i32,
    /// Relative speaking rate in the inclusive range `-1.0..=1.0`.
    pub rate: f32,
    /// Relative output volume in the inclusive range `-1.0..=1.0`.
    pub volume: f32,
}

impl Default for ReadAloudSpeechOptions {
    fn default() -> Self {
        Self {
            size: mem::size_of::<Self>() as u32,
            pitch_hz: 0,
            rate: 0.0,
            volume: 0.0,
        }
    }
}

const SPEECH_OPTIONS_SIZE_OFFSET: usize = std::mem::offset_of!(ReadAloudSpeechOptions, size);
const SPEECH_OPTIONS_PITCH_OFFSET: usize = std::mem::offset_of!(ReadAloudSpeechOptions, pitch_hz);
const SPEECH_OPTIONS_RATE_OFFSET: usize = std::mem::offset_of!(ReadAloudSpeechOptions, rate);
const SPEECH_OPTIONS_VOLUME_OFFSET: usize = std::mem::offset_of!(ReadAloudSpeechOptions, volume);
const MINIMUM_SPEECH_OPTIONS_SIZE: u32 =
    (SPEECH_OPTIONS_SIZE_OFFSET + mem::size_of::<u32>()) as u32;

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

fn parse_options(ptr: *const ReadAloudSpeechOptions) -> Result<SpeechOptions, TTSError> {
    if ptr.is_null() {
        return Ok(SpeechOptions::default());
    }

    let options = unsafe { &*ptr };
    if options.size < MINIMUM_SPEECH_OPTIONS_SIZE {
        return Err(TTSError::InvalidInput(
            "speech options size is smaller than the supported ABI".into(),
        ));
    }

    let mut parsed = SpeechOptions::default();
    let base = ptr.cast::<u8>();

    if has_field(options.size, SPEECH_OPTIONS_PITCH_OFFSET, mem::size_of::<i32>()) {
        parsed.pitch_hz = unsafe { ptr::read_unaligned(base.add(SPEECH_OPTIONS_PITCH_OFFSET).cast()) };
    }
    if has_field(options.size, SPEECH_OPTIONS_RATE_OFFSET, mem::size_of::<f32>()) {
        parsed.rate = unsafe { ptr::read_unaligned(base.add(SPEECH_OPTIONS_RATE_OFFSET).cast()) };
    }
    if has_field(options.size, SPEECH_OPTIONS_VOLUME_OFFSET, mem::size_of::<f32>()) {
        parsed.volume = unsafe { ptr::read_unaligned(base.add(SPEECH_OPTIONS_VOLUME_OFFSET).cast()) };
    }

    Ok(parsed)
}

fn has_field(size: u32, offset: usize, field_size: usize) -> bool {
    size as usize >= offset + field_size
}

#[no_mangle]
/// Return a short static description for a status code.
///
/// The returned pointer is always valid and must not be freed by the caller.
pub extern "C" fn read_aloud_status_string(status: ReadAloudStatus) -> *const c_char {
    static_status_message(status)
}

#[no_mangle]
/// Return the detailed message for the last failure on the calling thread.
///
/// The returned pointer remains valid until the next library call on the same thread. If no error
/// has been recorded, this function returns a pointer to an empty string.
pub extern "C" fn read_aloud_last_error_message() -> *const c_char {
    LAST_ERROR_MESSAGE.with(|slot| {
        let slot = slot.borrow();
        slot.as_ref()
            .map(|message| message.as_ptr())
            .unwrap_or_else(|| b"\0".as_ptr().cast())
    })
}

#[no_mangle]
/// Fill a caller-provided options struct with library defaults and the current ABI size.
///
/// Returns [ReadAloudStatus::InvalidInput] if `options` is null.
pub extern "C" fn read_aloud_speech_options_init(
    options: *mut ReadAloudSpeechOptions,
) -> ReadAloudStatus {
    clear_last_error();

    if options.is_null() {
        return record_error(TTSError::NullPointer("speech options".into()));
    }

    unsafe {
        *options = ReadAloudSpeechOptions::default();
    }

    ReadAloudStatus::Success
}

#[no_mangle]
/// Generate speech audio from UTF-8 `text` and write the result to `file_path`.
///
/// `text` and `file_path` must be non-null, NUL-terminated UTF-8 strings. `options` may be null,
/// in which case the library uses the default pitch, rate, and volume. On failure, use
/// [read_aloud_last_error_message] for a thread-local diagnostic message.
pub extern "C" fn read_aloud_text_to_speech(
    text: *const c_char,
    voice: Voice,
    options: *const ReadAloudSpeechOptions,
    file_path: *const c_char,
) -> ReadAloudStatus {
    clear_last_error();

    let result = std::panic::catch_unwind(|| {
        let text = parse_c_string(text, "text")?;
        let options = parse_options(options)?;
        let file_path = parse_c_string(file_path, "output path")?;
        super::text_to_speech(text, voice, options, Path::new(file_path))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem;

    #[repr(C)]
    struct SizeOnlyOptions {
        size: u32,
    }

    #[repr(C)]
    struct PitchOnlyOptions {
        size: u32,
        pitch_hz: i32,
    }

    #[test]
    fn parse_options_uses_defaults_for_null() {
        let parsed = parse_options(std::ptr::null()).expect("null options should use defaults");
        assert_eq!(parsed, SpeechOptions::default());
    }

    #[test]
    fn parse_options_rejects_too_small_size() {
        let options = SizeOnlyOptions { size: 0 };
        let error = parse_options((&options as *const SizeOnlyOptions).cast())
            .expect_err("too-small size should be rejected");
        assert!(matches!(error, TTSError::InvalidInput(_)));
    }

    #[test]
    fn parse_options_accepts_prefix_struct_and_defaults_missing_fields() {
        let options = PitchOnlyOptions {
            size: mem::size_of::<PitchOnlyOptions>() as u32,
            pitch_hz: 12,
        };

        let parsed = parse_options((&options as *const PitchOnlyOptions).cast())
            .expect("prefix struct should be accepted");

        assert_eq!(
            parsed,
            SpeechOptions {
                pitch_hz: 12,
                ..SpeechOptions::default()
            }
        );
    }

    #[test]
    fn parse_options_reads_all_known_fields() {
        let options = ReadAloudSpeechOptions {
            size: mem::size_of::<ReadAloudSpeechOptions>() as u32,
            pitch_hz: 4,
            rate: 0.25,
            volume: -0.5,
        };

        let parsed = parse_options(&options).expect("full struct should parse");
        assert_eq!(
            parsed,
            SpeechOptions {
                pitch_hz: 4,
                rate: 0.25,
                volume: -0.5,
            }
        );
    }

    #[test]
    fn speech_options_init_sets_defaults_and_size() {
        let mut options = ReadAloudSpeechOptions {
            size: 0,
            pitch_hz: 10,
            rate: 1.0,
            volume: 1.0,
        };

        let status = read_aloud_speech_options_init(&mut options);

        assert_eq!(status, ReadAloudStatus::Success);
        assert_eq!(options.size, mem::size_of::<ReadAloudSpeechOptions>() as u32);
        assert_eq!(options.pitch_hz, 0);
        assert_eq!(options.rate, 0.0);
        assert_eq!(options.volume, 0.0);
    }

    #[test]
    fn speech_options_init_rejects_null_pointer() {
        let status = read_aloud_speech_options_init(std::ptr::null_mut());
        assert_eq!(status, ReadAloudStatus::InvalidInput);
    }
}
