use super::*;

// ----- click classification --------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Surface {
    Desktop,
    Other,
}

pub(super) struct Classified {
    pub(super) surface: Surface,
    /// The window directly under the pointer is a SysListView32 (icon list).
    pub(super) hovered_is_list_view: bool,
    /// Class-name chain for the debug panel; only built when requested.
    pub(super) chain: String,
}

fn utf16_eq(units: &[u16], text: &str) -> bool {
    units.iter().copied().eq(text.encode_utf16())
}

/// Class names observed while walking from the hovered window to its ancestors.
/// Keeping the matching here makes the actual Win32 class-name handling testable
/// without creating real desktop or Explorer windows.
#[derive(Default)]
struct DesktopClassFlags {
    saw_progman: bool,
    saw_workerw: bool,
    saw_shell_view: bool,
    saw_explorer: bool,
}

impl DesktopClassFlags {
    fn observe(&mut self, name: &[u16]) {
        self.saw_progman |= utf16_eq(name, "Progman");
        self.saw_workerw |= utf16_eq(name, "WorkerW");
        self.saw_shell_view |= utf16_eq(name, "SHELLDLL_DefView");
        self.saw_explorer |=
            utf16_eq(name, "CabinetWClass") || utf16_eq(name, "ExploreWClass");
    }

    fn is_desktop_surface(&self) -> bool {
        (self.saw_progman || self.saw_workerw)
            && self.saw_shell_view
            && !self.saw_explorer
    }
}

/// Walks the ancestor chain of `start` and identifies the desktop icon view.
///
/// Windows may host the icon view under Progman or under a WorkerW window
/// (for example with slideshow wallpaper), so both host classes are supported.
///
/// # Safety
/// `start` must be null or a window handle.
pub(super) unsafe fn classify_window(start: HWND, want_chain: bool) -> Classified {
    if start.is_null() {
        return Classified {
            surface: Surface::Other,
            hovered_is_list_view: false,
            chain: "No window at pointer".to_owned(),
        };
    }

    let mut chain = Vec::new(); // stays unallocated unless `want_chain`
    let mut class_flags = DesktopClassFlags::default();
    let mut hovered_is_list_view = false;
    let mut buffer = [0u16; 64];
    let mut window = start;

    for depth in 0..MAX_ANCESTOR_DEPTH {
        // SAFETY: `window` is non-null here and `buffer` is a valid writable buffer.
        let length = unsafe { GetClassNameW(window, buffer.as_mut_ptr(), buffer.len() as i32) };
        let name = &buffer[..length.max(0) as usize];

        class_flags.observe(name);
        if depth == 0 {
            hovered_is_list_view = utf16_eq(name, "SysListView32");
        }
        if want_chain {
            chain.push(String::from_utf16_lossy(name));
        }

        // SAFETY: `window` is a valid window handle.
        window = unsafe { GetParent(window) };
        if window.is_null() {
            break;
        }
    }

    Classified {
        surface: if class_flags.is_desktop_surface() {
            Surface::Desktop
        } else {
            Surface::Other
        },
        hovered_is_list_view,
        chain: chain.join(" > "),
    }
}


#[cfg(test)]
mod classification_tests {
    use super::DesktopClassFlags;

    fn classify(classes: &[&str]) -> bool {
        let mut flags = DesktopClassFlags::default();
        for class in classes {
            flags.observe(&class.encode_utf16().collect::<Vec<_>>());
        }
        flags.is_desktop_surface()
    }

    #[test]
    fn recognizes_progman_desktop_view_in_ancestor_chain() {
        assert!(classify(&["SysListView32", "SHELLDLL_DefView", "Progman"]));
    }

    #[test]
    fn recognizes_workerw_desktop_view_in_ancestor_chain() {
        assert!(classify(&["SysListView32", "SHELLDLL_DefView", "WorkerW"]));
    }

    #[test]
    fn recognizes_workerw_when_progman_is_not_in_chain() {
        assert!(classify(&["SHELLDLL_DefView", "WorkerW"]));
    }

    #[test]
    fn rejects_workerw_without_shell_view() {
        assert!(!classify(&["SysListView32", "WorkerW"]));
    }

    #[test]
    fn rejects_shell_view_without_desktop_host() {
        assert!(!classify(&["SysListView32", "SHELLDLL_DefView"]));
    }

    #[test]
    fn rejects_file_explorer_ancestor_even_with_desktop_markers() {
        assert!(!classify(&[
            "SysListView32",
            "SHELLDLL_DefView",
            "Progman",
            "CabinetWClass",
        ]));
    }

    #[test]
    fn rejects_explorer_window_classes() {
        assert!(!classify(&["SysListView32", "ExploreWClass"]));
    }
}

// ----- cross-process icon hit-test (RAII wrappers) ---------------------

#[repr(C)]
struct LvHitTestInfo {
    pt: POINT,
    flags: u32,
    i_item: i32,
    i_sub_item: i32,
    i_group: i32,
}

struct ProcessHandle(HANDLE);

impl ProcessHandle {
    fn open_for_memory_access(process_id: u32) -> Option<Self> {
        // SAFETY: plain FFI call; a null result is handled.
        let handle = unsafe {
            OpenProcess(
                PROCESS_VM_OPERATION | PROCESS_VM_READ | PROCESS_VM_WRITE,
                0,
                process_id,
            )
        };
        (!handle.is_null()).then_some(Self(handle))
    }
}

impl Drop for ProcessHandle {
    fn drop(&mut self) {
        // SAFETY: the handle came from OpenProcess and is closed exactly once.
        unsafe { CloseHandle(self.0) };
    }
}

/// A buffer allocated inside another process; freed on drop.
/// Declare it *after* the `ProcessHandle` it borrows so it drops first.
struct RemoteAlloc {
    process: HANDLE,
    ptr: *mut c_void,
}

impl RemoteAlloc {
    fn new(process: &ProcessHandle, size: usize) -> Option<Self> {
        // SAFETY: plain FFI call on a valid process handle; null is handled.
        let ptr = unsafe {
            VirtualAllocEx(
                process.0,
                std::ptr::null(),
                size,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_READWRITE,
            )
        };
        (!ptr.is_null()).then_some(Self {
            process: process.0,
            ptr,
        })
    }
}

impl Drop for RemoteAlloc {
    fn drop(&mut self) {
        // SAFETY: `ptr` was allocated by VirtualAllocEx in `process` and is freed once.
        unsafe { VirtualFreeEx(self.process, self.ptr, 0, MEM_RELEASE) };
    }
}

/// True if `screen_point` lands on an actual icon inside the SysListView32
/// `list_view`, rather than on empty desktop space.
///
/// The list view belongs to explorer.exe, so LVM_HITTEST's LPARAM can't point at a
/// struct in *our* address space (explorer would dereference it in *its* space and
/// crash). We allocate the struct inside explorer, send the message with a timeout
/// (so a hung explorer can't block us), read the result back, and free it.
///
/// Any failure returns `false` ("not an icon"), matching the old behaviour.
///
/// # Safety
/// `list_view` must be a SysListView32 window handle.
pub(super) unsafe fn click_hits_icon(list_view: HWND, screen_point: POINT) -> bool {
    let mut client_point = screen_point;
    // SAFETY: valid window handle and a valid out-pointer.
    if unsafe { ScreenToClient(list_view, &mut client_point) } == 0 {
        return false;
    }

    let mut process_id = 0u32;
    // SAFETY: valid window handle and a valid out-pointer.
    unsafe { GetWindowThreadProcessId(list_view, &mut process_id) };
    if process_id == 0 {
        return false;
    }

    let Some(process) = ProcessHandle::open_for_memory_access(process_id) else {
        return false;
    };
    let size = std::mem::size_of::<LvHitTestInfo>();
    let Some(remote) = RemoteAlloc::new(&process, size) else {
        return false;
    };

    let mut info = LvHitTestInfo {
        pt: client_point,
        flags: 0,
        i_item: -1,
        i_sub_item: 0,
        i_group: 0,
    };

    // SAFETY: `remote` is `size` bytes of committed memory in `process`; `info` is
    // a valid local `repr(C)` struct of the same size.
    unsafe {
        let written = WriteProcessMemory(
            process.0,
            remote.ptr,
            &info as *const LvHitTestInfo as *const c_void,
            size,
            std::ptr::null_mut(),
        );
        if written == 0 {
            return false;
        }

        let mut message_result = 0usize;
        let sent = SendMessageTimeoutW(
            list_view,
            LVM_HITTEST,
            0,
            remote.ptr as LPARAM,
            SMTO_ABORTIFHUNG,
            HIT_TEST_TIMEOUT_MS,
            &mut message_result,
        );
        if sent == 0 {
            return false;
        }

        let read = ReadProcessMemory(
            process.0,
            remote.ptr,
            &mut info as *mut LvHitTestInfo as *mut c_void,
            size,
            std::ptr::null_mut(),
        );
        if read == 0 {
            return false;
        }
    }

    info.i_item >= 0
}
