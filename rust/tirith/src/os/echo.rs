//! The terminal's echo off while a secret is typed, back as it was when this ends.

/// The terminal as it was before the echo went off.
#[cfg(unix)]
pub struct NoEcho(libc::termios);

#[cfg(windows)]
pub struct NoEcho(u32);

#[cfg(unix)]
impl NoEcho {
    pub fn new() -> Option<NoEcho> {
        // SAFETY: an all-zero termios is valid, and tcgetattr fills it before it is used.
        unsafe {
            let mut was: libc::termios = std::mem::zeroed();
            (libc::tcgetattr(0, &mut was) == 0).then_some(())?;
            let mut now = was;
            now.c_lflag &= !libc::ECHO;
            (libc::tcsetattr(0, libc::TCSANOW, &now) == 0).then_some(NoEcho(was))
        }
    }
}

#[cfg(unix)]
impl Drop for NoEcho {
    fn drop(&mut self) {
        unsafe { libc::tcsetattr(0, libc::TCSANOW, &self.0) };
    }
}

#[cfg(windows)]
impl NoEcho {
    pub fn new() -> Option<NoEcho> {
        use windows_sys::Win32::System::Console::{
            GetConsoleMode, SetConsoleMode, ENABLE_ECHO_INPUT,
        };
        // SAFETY: the console's input handle, and a mode written into a local.
        unsafe {
            let mut was = 0u32;
            (GetConsoleMode(input(), &mut was) != 0).then_some(())?;
            (SetConsoleMode(input(), was & !ENABLE_ECHO_INPUT) != 0).then_some(NoEcho(was))
        }
    }
}

#[cfg(windows)]
impl Drop for NoEcho {
    fn drop(&mut self) {
        // SAFETY: the mode the console had before.
        unsafe { windows_sys::Win32::System::Console::SetConsoleMode(input(), self.0) };
    }
}

/// The console's input.
#[cfg(windows)]
fn input() -> windows_sys::Win32::Foundation::HANDLE {
    use windows_sys::Win32::System::Console::{GetStdHandle, STD_INPUT_HANDLE};
    // SAFETY: asks for a handle the process already holds.
    unsafe { GetStdHandle(STD_INPUT_HANDLE) }
}
