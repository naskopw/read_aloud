import ctypes
import os

lib_path = os.path.abspath('./target/debug/libread_aloud.so')
lib = ctypes.CDLL(lib_path)

voice = 223  # en_GB_ThomasNeural

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

text = b"Hello, World!"
pitch = 0
rate = ctypes.c_float(0.0)
volume = ctypes.c_float(0.0)
file = b"smoke_output.mp3"

result = lib.text_to_speech(text, voice, pitch, rate, volume, file)
if result != 0:
    status = lib.read_aloud_status_string(result).decode()
    details = lib.read_aloud_last_error_message().decode()
    print(f"text_to_speech failed with status code {result}: {status}")
    print(f"details: {details}")
else:
    print(f"Text to speech succeeded, output saved to {file.decode()}")
