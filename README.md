# Read aloud

A cross-platform text-to-speech library with C interface, written in Rust.

It reverse-engineers the awesome [Microsoft Edge Read aloud](https://www.microsoft.com/en-us/edge/features/read-aloud?) feature, allowing you to use it in your own projects in your preferred programming language.

Read aloud is a low-level library that is designed as a building block for higher-level text-to-speech libraries and applications.


## Edge service compatibility

> This crate counterfeits the communication protocol used by Microsoft Edge to work with the Read Aloud service. **Any change to the upstream Edge service can break the crate, even if the public C ABI stays the same.** If the service updates its protocol, authentication, or required headers, the library may need internal changes to restore functionality.

### Quick verification

After building the library, you can run the bundled Python smoke test:

```sh
cargo build && python smoke.py
```

If the request succeeds, it writes `output.mp3` in the repository root.

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


The library exposes a stable C ABI for text-to-speech generation and error reporting.

```c
// Generate speech audio from text and save to a file. Returns a status code.
enum ReadAloudStatus text_to_speech(
    const char *text,
    enum Voice voice,
    int32_t pitch,
    float rate,
    float volume,
    const char *output_path
);

// Get a short static description for a status code.
const char *read_aloud_status_string(enum ReadAloudStatus status);

// Get a detailed error message for the last failure on the calling thread.
const char *read_aloud_last_error_message(void);
```

`text_to_speech` returns a stable numeric status code.

`read_aloud_status_string` returns a short static description for that code.

`read_aloud_last_error_message` returns a more detailed, thread-local message describing the last failure on the calling thread. The pointer remains valid until the next library call on the same thread.

### Parameters

`pitch` is specified in Hz.

`rate` must be between `-1.0` and `1.0`, where `0.0` is the default voice speed.

`volume` must be between `-1.0` and `1.0`, where `0.0` is the default voice volume.

### Error codes

The function returns 0 on success, and a non-zero error code on failure:

[Error codes](./src/ffi.rs#L16)

## Supported languages and voices

[Languages and voices](./src/voices.rs)

## Examples

## C++

```cpp
// Calling text_to_speech from C++ on Windows
#include <iostream>
#include <windows.h>
#include "read_aloud.h"

typedef ReadAloudStatus (*TextToSpeechFunc)(const char *, Voice, int, float, float, const char *);
typedef const char *(*StatusStringFunc)(ReadAloudStatus);
typedef const char *(*LastErrorFunc)();

int main()
{
    HMODULE hModule = LoadLibraryA("read_aloud.dll");
    if (!hModule)
    {
        std::cerr << "Failed to load read_aloud.dll" << std::endl;
        return 1;
    }

    TextToSpeechFunc text_to_speech = (TextToSpeechFunc)GetProcAddress(hModule, "text_to_speech");
    StatusStringFunc status_string = (StatusStringFunc)GetProcAddress(hModule, "read_aloud_status_string");
    LastErrorFunc last_error = (LastErrorFunc)GetProcAddress(hModule, "read_aloud_last_error_message");
    if (!text_to_speech || !status_string || !last_error)
    {
        std::cerr << "Failed to get function addresses" << std::endl;
        FreeLibrary(hModule);
        return 1;
    }

    const char *text = "Hello, World!";
    Voice voice = en_GB_ThomasNeural;
    int pitch = 0;
    float rate = 0.0f;
    float volume = 0.0f;
    const char *file = "output.mp3";

    ReadAloudStatus result = text_to_speech(text, voice, pitch, rate, volume, file);
    if (result != Success)
    {
        std::cerr << "text_to_speech failed: " << status_string(result) << std::endl;
        std::cerr << "details: " << last_error() << std::endl;
    }
    else
    {
        std::cout << "Text to speech succeeded, output saved to " << file << std::endl;
    }

    FreeLibrary(hModule);
    return 0;
}
```

## Python

```python
import ctypes
import os

lib = ctypes.CDLL(os.path.abspath("./target/debug/libread_aloud.so"))

lib.text_to_speech.argtypes = [
    ctypes.c_char_p,
    ctypes.c_int,
    ctypes.c_int,
    ctypes.c_float,
    ctypes.c_float,
    ctypes.c_char_p,
]
lib.text_to_speech.restype = ctypes.c_int

lib.read_aloud_status_string.argtypes = [ctypes.c_int]
lib.read_aloud_status_string.restype = ctypes.c_char_p
lib.read_aloud_last_error_message.argtypes = []
lib.read_aloud_last_error_message.restype = ctypes.c_char_p

voice = 223  # en_GB_ThomasNeural
result = lib.text_to_speech(
    b"Hello, World!",
    voice,
    0,
    ctypes.c_float(0.0),
    ctypes.c_float(0.0),
    b"output.mp3",
)

if result != 0:
    print("status:", lib.read_aloud_status_string(result).decode())
    print("details:", lib.read_aloud_last_error_message().decode())
else:
    print("Text to speech succeeded")
```
