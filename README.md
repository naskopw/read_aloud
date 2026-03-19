# Read aloud

A cross-platform text-to-speech library with C interface, written in Rust.

It reverse-engineers the awesome [Microsoft Edge Read aloud](https://www.microsoft.com/en-us/edge/features/read-aloud?) feature, allowing you to use it in your own projects in your preferred programming language.

Read aloud is a low-level library that is designed as a building block for higher-level text-to-speech libraries and applications.


## Edge service compatibility

> This crate counterfeits the communication protocol used by Microsoft Edge to work with the Read Aloud service. **Any change to the upstream Edge service can break the crate, even if the public C ABI stays the same.** If the service updates its protocol, authentication, or required headers, the library may need internal changes to restore functionality.

## Build requirements

This library connects to the Edge read-aloud service over secure WebSockets. TLS support is currently provided by `tungstenite` with its `native-tls` feature enabled.

On Linux, `native-tls` uses the system OpenSSL installation, so building this crate requires OpenSSL development files and `pkg-config` to be installed.

For Debian or Ubuntu:

```sh
sudo apt-get update
sudo apt-get install libssl-dev pkg-config
```

On Windows and macOS, `native-tls` uses the platform TLS stack instead of OpenSSL, so this Linux-specific package installation is usually not required.

## API

The library exposes both a Rust API and a C ABI for text-to-speech generation.

### Rust API

```rust
use std::path::Path;

use read_aloud::{text_to_speech, SpeechOptions, Voice};

let options = SpeechOptions {
    pitch_hz: 0,
    rate: 0.0,
    volume: 0.0,
};

text_to_speech(
    "Hello, World!",
    Voice::en_GB_ThomasNeural,
    options,
    Path::new("output.mp3"),
)?;
```

`SpeechOptions::default()` uses the service defaults for pitch, rate, and volume.

### C API

The C ABI uses a size-prefixed options struct so the parameter surface can evolve without redesigning the function signature.

```c
typedef struct ReadAloudSpeechOptions {
    uint32_t size;
    int32_t pitch_hz;
    float rate;
    float volume;
} ReadAloudSpeechOptions;

// Initialize an options struct with library defaults.
enum ReadAloudStatus read_aloud_speech_options_init(ReadAloudSpeechOptions *options);

// Generate speech audio from text and save to a file. Pass NULL for default options.
enum ReadAloudStatus read_aloud_text_to_speech(
    const char *text,
    enum Voice voice,
    const ReadAloudSpeechOptions *options,
    const char *output_path
);

// Get a short static description for a status code.
const char *read_aloud_status_string(enum ReadAloudStatus status);

// Get a detailed error message for the last failure on the calling thread.
const char *read_aloud_last_error_message(void);
```

`read_aloud_text_to_speech` returns a numeric status code.

Passing `NULL` for `options` uses the default pitch, rate, and volume.

`read_aloud_speech_options_init` fills a caller-provided options struct and sets its `size` field for the current ABI.

`read_aloud_status_string` returns a short static description for that code.

`read_aloud_last_error_message` returns a more detailed, thread-local message describing the last failure on the calling thread. The pointer remains valid until the next library call on the same thread.

### Parameters

`pitch_hz` is specified in Hz.

`rate` must be between `-1.0` and `1.0`, where `0.0` is the default voice speed.

`volume` must be between `-1.0` and `1.0`, where `0.0` is the default voice volume.

### Error codes

The function returns 0 on success, and a non-zero error code on failure:

[Error codes](./src/ffi.rs#L16)

## Supported languages and voices

[Languages and voices](./src/voices.rs)

## Examples

<details>
<summary>C</summary>

```c
// Load the read_aloud shared library and resolve the exported symbols before running this example.
#include <stdio.h>
#include "read_aloud.h"

int main(void) {
    enum ReadAloudStatus status = read_aloud_text_to_speech(
        "Hello, World!",
        en_GB_ThomasNeural,
        NULL,
        "output.mp3"
    );
    if (status != Success) {
        fprintf(stderr, "TTS failed: %s\n", read_aloud_status_string(status));
        fprintf(stderr, "details: %s\n", read_aloud_last_error_message());
        return 1;
    }

    ReadAloudSpeechOptions options;
    status = read_aloud_speech_options_init(&options);
    if (status != Success) {
        fprintf(stderr, "Failed to initialize options: %s\n", read_aloud_last_error_message());
        return 1;
    }

    options.rate = 0.2f;
    status = read_aloud_text_to_speech(
        "Custom rate!",
        en_GB_ThomasNeural,
        &options,
        "custom_output.mp3"
    );
    if (status != Success) {
        fprintf(stderr, "TTS failed: %s\n", read_aloud_status_string(status));
        fprintf(stderr, "details: %s\n", read_aloud_last_error_message());
        return 1;
    }

    printf("Both TTS calls succeeded.\n");
    return 0;
}
```

</details>

<details>
<summary>Rust</summary>

```rust
use std::path::Path;

use read_aloud::{text_to_speech, SpeechOptions, Voice};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    text_to_speech(
        "Hello, World!",
        Voice::en_GB_ThomasNeural,
        SpeechOptions::default(),
        Path::new("output.mp3"),
    )?;

    let mut options = SpeechOptions::default();
    options.rate = 0.2;

    text_to_speech(
        "Custom rate!",
        Voice::en_GB_ThomasNeural,
        options,
        Path::new("custom_output.mp3"),
    )?;

    Ok(())
}
```

</details>

<details>
<summary>C++</summary>

```cpp
// Load the read_aloud shared library and resolve the exported symbols before running this example.
#include <iostream>
#include "read_aloud.h"

int main() {
    auto status = read_aloud_text_to_speech("Hello, World!", en_GB_ThomasNeural, nullptr, "output.mp3");
    if (status != Success) {
        std::cerr << "TTS failed: " << read_aloud_status_string(status) << std::endl;
        std::cerr << "details: " << read_aloud_last_error_message() << std::endl;
        return 1;
    }

    ReadAloudSpeechOptions options;
    status = read_aloud_speech_options_init(&options);
    if (status != Success) {
        std::cerr << "Failed to initialize options: " << read_aloud_last_error_message() << std::endl;
        return 1;
    }

    options.rate = 0.2f;
    status = read_aloud_text_to_speech("Custom rate!", en_GB_ThomasNeural, &options, "custom_output.mp3");
    if (status != Success) {
        std::cerr << "TTS failed: " << read_aloud_status_string(status) << std::endl;
        std::cerr << "details: " << read_aloud_last_error_message() << std::endl;
        return 1;
    }

    std::cout << "Both TTS calls succeeded." << std::endl;
    return 0;
}
```

</details>

<details>
<summary>Python</summary>

```python
import ctypes

# Load the read_aloud shared library before running this example.
# Assume `lib` is an already-loaded ctypes.CDLL instance.

class ReadAloudSpeechOptions(ctypes.Structure):
    _fields_ = [
        ("size", ctypes.c_uint32),
        ("pitch_hz", ctypes.c_int32),
        ("rate", ctypes.c_float),
        ("volume", ctypes.c_float),
    ]

lib.read_aloud_speech_options_init.argtypes = [ctypes.POINTER(ReadAloudSpeechOptions)]
lib.read_aloud_speech_options_init.restype = ctypes.c_int

lib.read_aloud_text_to_speech.argtypes = [
    ctypes.c_char_p,
    ctypes.c_int,
    ctypes.POINTER(ReadAloudSpeechOptions),
    ctypes.c_char_p,
]
lib.read_aloud_text_to_speech.restype = ctypes.c_int

lib.read_aloud_status_string.argtypes = [ctypes.c_int]
lib.read_aloud_status_string.restype = ctypes.c_char_p
lib.read_aloud_last_error_message.argtypes = []
lib.read_aloud_last_error_message.restype = ctypes.c_char_p


def print_error(status: int) -> None:
    print("status:", lib.read_aloud_status_string(status).decode())
    print("details:", lib.read_aloud_last_error_message().decode())


voice = 110  # en_GB_ThomasNeural

result = lib.read_aloud_text_to_speech(
    b"Hello, World!",
    voice,
    None,
    b"output.mp3",
)
if result != 0:
    print_error(result)
    raise SystemExit(1)

options = ReadAloudSpeechOptions()
result = lib.read_aloud_speech_options_init(ctypes.byref(options))
if result != 0:
    print_error(result)
    raise SystemExit(1)

options.rate = ctypes.c_float(0.2)
result = lib.read_aloud_text_to_speech(
    b"Custom rate!",
    voice,
    ctypes.byref(options),
    b"custom_output.mp3",
)
if result != 0:
    print_error(result)
    raise SystemExit(1)

print("Both TTS calls succeeded.")
```

</details>


