//! Sends the log to Android's logcat (tag `Kaze`). Android discards stdout, so
//! without this a release build's warnings, such as a failed sync, are lost.

use std::ffi::CString;
use std::io::{self, Write};
use std::os::raw::{c_char, c_int};

const TAG: &[u8] = b"Kaze\0";
const INFO: c_int = 4;

#[link(name = "log")]
extern "C" {
    fn __android_log_write(priority: c_int, tag: *const c_char, text: *const c_char) -> c_int;
}

/// One log line per write; the level is already in the formatted text.
pub struct Logcat;

impl Write for Logcat {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let text = String::from_utf8_lossy(buf);
        let line = text.trim_end().replace('\0', " ");
        if let Ok(line) = CString::new(line) {
            // SAFETY: both pointers are NUL-terminated and live for the call.
            unsafe { __android_log_write(INFO, TAG.as_ptr().cast(), line.as_ptr()) };
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
