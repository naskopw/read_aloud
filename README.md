# Read aloud

A cross-platform text-to-speech library with C interface, written in Rust.

It reverse-engineers the awesome [Microsoft Edge Read aloud](https://www.microsoft.com/en-us/edge/features/read-aloud?) feature, allowing you to use it in your own projects in your preferred programming language.

Read aloud is a low-level library that is designed as a building block for higher-level text-to-speech libraries and applications.


## Edge service compatibility

> This crate counterfeits the communication protocol used by Microsoft Edge to work with the Read Aloud service. **Any change to the upstream Edge service can break the crate, even if the public C ABI stays the same.** If the service updates its protocol, authentication, or required headers, the library may need internal changes to restore functionality.

## Build and integration

### Build requirements

This library connects to the Edge read-aloud service over secure WebSockets. TLS support is currently provided by `tungstenite` with its `native-tls` feature enabled.

<details>
<summary>Linux</summary>
On Linux, `native-tls` uses the system OpenSSL installation, so building this crate requires OpenSSL development files and `pkg-config` to be installed.

For Debian or Ubuntu:

```sh
sudo apt-get update
sudo apt-get install libssl-dev pkg-config
```
</details>

<details>
<summary>Windows/macOS</summary>
On Windows and macOS, `native-tls` uses the platform TLS stack instead of OpenSSL, so this Linux-specific package installation is usually not required.
</details>

### Integration

#### Rust

Add the crate to your Rust project:

```sh
cargo add read-aloud
```

The crate builds as a normal Rust library, so `cargo build` is enough when you only need the Rust API. For usage, see the [API Reference](#api-reference) and [Examples](#examples) sections below.

#### C

Build the shared library and generated header with Cargo:

```sh
cargo build --release
```

This crate is configured to produce a shared library (`cdylib`) and a Rust library (`rlib`). After a release build, the generated artifacts are placed in Cargo's target directory. By default this is `target/release/` for the host platform, or `target/<target-triple>/release/` for cross-compilation.

The C integration artifacts are:

- Shared library: `libread_aloud.so` on Linux, `libread_aloud.dylib` on macOS, or `read_aloud.dll` on Windows
- Generated header: `read_aloud.h`

Link the shared library from your C or C++ project and include the generated header from the same target directory.

The generated header includes API comments sourced from the Rust rustdoc on the exported FFI items, so the Rust sources are the canonical reference for the C ABI.

## API

The library exposes both a Rust API and a C ABI for text-to-speech generation.

### API Reference

See the full API documentation at: https://docs.rs/crate/read-aloud/latest

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


