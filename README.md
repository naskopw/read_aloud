# Read aloud

A cross-platform text-to-speech library with C interface, written in Rust.

It reverse-engineers the awesome [Microsoft Edge Read aloud](https://www.microsoft.com/en-us/edge/features/read-aloud?) feature, allowing you to use it in your own projects in your preferred programming language.

Read aloud is a low-level library that is designed as a building block for higher-level text-to-speech libraries and applications.

## API

The library provides a single function that takes a text, a voice, and a file path as input, and saves the output to the file in MP3 format.

```c
int text_to_speech(const char *text, enum Voice voice, const char *f);
```

### Error codes

The function returns 0 on success, and a non-zero error code on failure:

[Error codes](./src/ffi.rs#L10)

## Supported languages and voices

[Languages and voices](./src/voices.rs)

# Example

```cpp
// Calling text_to_speech from C++ on Windows
#include <iostream>
#include <windows.h>
#include "read_aloud.h"

typedef int (*TextToSpeechFunc)(const char *, Voice, const char *);

int main()
{
    HMODULE hModule = LoadLibraryA("read_aloud.dll");
    if (!hModule)
    {
        std::cerr << "Failed to load read_aloud.dll" << std::endl;
        return 1;
    }

    TextToSpeechFunc text_to_speech = (TextToSpeechFunc)GetProcAddress(hModule, "text_to_speech");
    if (!text_to_speech)
    {
        std::cerr << "Failed to get text_to_speech function address" << std::endl;
        FreeLibrary(hModule);
        return 1;
    }

    const char *text = "Hello, World!";
    Voice voice = en_GB_ThomasNeural;
    const char *file = "output.mp3";

    int result = text_to_speech(text, voice, file);
    if (result != 0)
    {
        std::cerr << "text_to_speech failed with error code " << result << std::endl;
    }
    else
    {
        std::cout << "Text to speech succeeded, output saved to " << file << std::endl;
    }

    FreeLibrary(hModule);
    return 0;
}
```
