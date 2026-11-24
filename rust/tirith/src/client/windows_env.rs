//! Another process's parent and environment on Windows, which tells neither without asking the kernel: the parent
//! from the process's basic information, the environment read out of its memory, from the block its process
//! parameters point at. The offsets are those of every 64-bit Windows since Vista (and of psutil's reader).
//! Only processes of the same user open, as on Unix; anything else is none.
use windows_sys::Wdk::System::Threading::{NtQueryInformationProcess, ProcessBasicInformation};
use windows_sys::Win32::Foundation::{CloseHandle, HANDLE};
use windows_sys::Win32::System::Diagnostics::Debug::ReadProcessMemory;
use windows_sys::Win32::System::Threading::{
    OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_VM_READ,
};

/// PROCESS_BASIC_INFORMATION, as NtQueryInformationProcess fills it.
#[repr(C)]
#[derive(Default)]
struct Basic {
    exit: i32,
    peb: usize,
    affinity: usize,
    priority: i32,
    pid: usize,
    parent: usize,
}

/// Where the PEB keeps its process parameters, and they the environment and its size.
const PARAMETERS: usize = 0x20;
const ENVIRONMENT: usize = 0x80;
const ENVIRONMENT_SIZE: usize = 0x3F0;
/// More than an environment holds: a wrong size never makes a huge read.
const MOST: usize = 1 << 20;

struct Open(HANDLE);

impl Drop for Open {
    fn drop(&mut self) {
        // SAFETY: a handle OpenProcess gave, closed once.
        unsafe { CloseHandle(self.0) };
    }
}

fn open(pid: u32, rights: u32) -> Option<Open> {
    // SAFETY: a plain call; a null handle is its failure.
    let handle = unsafe { OpenProcess(rights, 0, pid) };
    (!handle.is_null()).then_some(Open(handle))
}

fn basic(process: &Open) -> Option<Basic> {
    let mut info = Basic::default();
    let size = std::mem::size_of::<Basic>() as u32;
    let at = (&raw mut info).cast();
    // SAFETY: `info` is the struct this class fills, of the size the call is told.
    let status = unsafe {
        NtQueryInformationProcess(
            process.0,
            ProcessBasicInformation,
            at,
            size,
            std::ptr::null_mut(),
        )
    };
    (status == 0).then_some(info)
}

fn read(process: &Open, at: usize, buf: &mut [u8]) -> Option<()> {
    let mut got = 0usize;
    let (to, len) = (buf.as_mut_ptr().cast(), buf.len());
    // SAFETY: the kernel writes at most `len` bytes into `buf`, and says how many it wrote.
    let ok = unsafe { ReadProcessMemory(process.0, at as *const _, to, len, &mut got) };
    (ok != 0 && got == len).then_some(())
}

fn word(process: &Open, at: usize) -> Option<usize> {
    let mut bytes = [0u8; std::mem::size_of::<usize>()];
    read(process, at, &mut bytes)?;
    Some(usize::from_le_bytes(bytes))
}

pub(super) fn parent(pid: u32) -> Option<u32> {
    let process = open(pid, PROCESS_QUERY_LIMITED_INFORMATION)?;
    u32::try_from(basic(&process)?.parent).ok()
}

/// Its environment as NUL-ended `NAME=value` words in UTF-8, as the Unix readers give it.
pub(super) fn given(pid: u32) -> Option<Vec<u8>> {
    let process = open(pid, PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ)?;
    let params = word(&process, basic(&process)?.peb + PARAMETERS)?;
    let env = word(&process, params + ENVIRONMENT)?;
    let size = word(&process, params + ENVIRONMENT_SIZE)?;
    let mut raw = vec![0u8; size.min(MOST) & !1];
    read(&process, env, &mut raw)?;
    let (pairs, _) = raw.as_chunks::<2>();
    let wide: Vec<u16> = pairs.iter().map(|p| u16::from_le_bytes(*p)).collect();
    Some(String::from_utf16_lossy(&wide).into_bytes())
}
