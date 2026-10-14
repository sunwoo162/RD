#![cfg_attr(windows, windows_subsystem = "windows")]

#[cfg(not(windows))]
fn main() {
    eprintln!("RD Tray는 Windows 전용 프로그램입니다.");
}

#[cfg(windows)]
mod app {
    use fs2::FileExt;
    use rd_agent::{config, model::Settings, sync::{self, SyncIntent}};
    use std::{
        collections::HashMap,
        fs::{self, File, OpenOptions},
        io::Write,
        path::Path,
        process::Command,
        sync::{mpsc::{self, Receiver, Sender}, OnceLock},
        thread,
        time::{Duration, Instant, SystemTime, UNIX_EPOCH},
    };
    use windows_sys::{
        w,
        Win32::{
            Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM},
            System::LibraryLoader::GetModuleHandleW,
            UI::{
                Shell::{Shell_NotifyIconW, NOTIFYICONDATAW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE},
                WindowsAndMessaging::{
                    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu,
                    DestroyWindow, DispatchMessageW, GetCursorPos, GetMessageW, LoadIconW,
                    PostMessageW, PostQuitMessage, RegisterClassW, SetForegroundWindow,
                    TrackPopupMenu, TranslateMessage, WNDCLASSW, MSG,
                    HWND_MESSAGE, IDI_APPLICATION, MF_SEPARATOR, MF_STRING,
                    TPM_RIGHTBUTTON, WM_APP, WM_COMMAND, WM_DESTROY, WM_LBUTTONDBLCLK,
                    WM_NULL, WM_RBUTTONUP,
                },
            },
        },
    };

    const TRAY_CALLBACK: u32 = WM_APP + 1;
    const MENU_CHECK: usize = 1001;
    const MENU_FOLDER: usize = 1002;
    const MENU_MANAGER: usize = 1004;
    const MENU_QUIT: usize = 1003;
    static ACTIONS: OnceLock<Sender<Action>> = OnceLock::new();

    #[derive(Clone, Copy)]
    enum Action { CheckNow, Quit }

    fn append_log(dir: &Path, message: &str) {
        let log = dir.join("agent.log");
        if fs::metadata(&log).is_ok_and(|m| m.len() > 1_000_000) {
            let _ = fs::rename(&log, dir.join("agent.old.log"));
        }
        if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(log) {
            let seconds = SystemTime::now().duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_secs());
            let _ = writeln!(file, "{} {}", seconds, message);
        }
    }

    fn worker(dir: &Path, rx: Receiver<Action>) {
        let mut due: HashMap<String, Instant> = HashMap::new();
        let mut immediate = false;
        loop {
            match config::load(dir) {
                Ok(Settings { interval_minutes, repositories, .. }) => {
                    let interval = Duration::from_secs(interval_minutes.max(1).saturating_mul(60));
                    let now = Instant::now();
                    for (index, repo) in repositories.iter().enumerate() {
                        if !repo.enabled { continue; }
                        let jitter = Duration::from_secs((index as u64 * 7) % interval.as_secs());
                        let deadline = due.entry(repo.id.clone()).or_insert(now + jitter);
                        if immediate || Instant::now() >= *deadline {
                            // The tray's manual "Check now" never modifies working files.
                            let intent = if immediate { SyncIntent::Check } else { SyncIntent::Scheduled };
                            match sync::inspect(repo, dir, intent) {
                                Ok(status) => append_log(dir, &format!("{}: {}", repo.id, status)),
                                // Never persist raw Git errors; stderr may contain credentials.
                                Err(_) => append_log(dir, &format!("{}: 동기화 검사 실패", repo.id)),
                            }
                            *deadline = Instant::now() + interval;
                        }
                    }
                    due.retain(|id, _| repositories.iter().any(|repo| &repo.id == id));
                }
                Err(_) => append_log(dir, "설정 파일을 읽을 수 없습니다"),
            }
            immediate = false;
            match rx.recv_timeout(Duration::from_secs(10)) {
                Ok(Action::CheckNow) => immediate = true,
                Ok(Action::Quit) | Err(mpsc::RecvTimeoutError::Disconnected) => break,
                Err(mpsc::RecvTimeoutError::Timeout) => {}
            }
        }
    }

    unsafe extern "system" fn window_proc(
        hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM,
    ) -> LRESULT {
        match message {
            TRAY_CALLBACK if lparam as u32 == WM_RBUTTONUP => {
                let menu = CreatePopupMenu();
                if !menu.is_null() {
                    AppendMenuW(menu, MF_STRING, MENU_MANAGER, w!("RD 관리 화면 열기"));
                    AppendMenuW(menu, MF_STRING, MENU_CHECK, w!("지금 변경 확인"));
                    AppendMenuW(menu, MF_STRING, MENU_FOLDER, w!("설정 폴더 열기"));
                    AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
                    AppendMenuW(menu, MF_STRING, MENU_QUIT, w!("RD 종료"));
                    let mut point = POINT::default();
                    if GetCursorPos(&mut point) != 0 {
                        SetForegroundWindow(hwnd);
                        TrackPopupMenu(menu, TPM_RIGHTBUTTON, point.x, point.y, 0, hwnd, std::ptr::null());
                        PostMessageW(hwnd, WM_NULL, 0, 0);
                    }
                    DestroyMenu(menu);
                }
                0
            }
            TRAY_CALLBACK if lparam as u32 == WM_LBUTTONDBLCLK => {
                open_manager();
                0
            }
            WM_COMMAND => {
                match wparam & 0xffff {
                    MENU_CHECK => { if let Some(tx) = ACTIONS.get() { let _ = tx.send(Action::CheckNow); } },
                    MENU_FOLDER => open_settings(),
                    MENU_MANAGER => open_manager(),
                    MENU_QUIT => { DestroyWindow(hwnd); },
                    _ => {}
                }
                0
            }
            WM_DESTROY => { PostQuitMessage(0); 0 }
            _ => DefWindowProcW(hwnd, message, wparam, lparam),
        }
    }

    fn open_manager() {
        if let Ok(exe) = std::env::current_exe() {
            let manager = exe.with_file_name("rd-manager.exe");
            if manager.is_file() {
                let _ = Command::new(manager).spawn();
                return;
            }
        }
        // CLI-only/older portable packages can still open their data folder.
        open_settings();
    }

    fn open_settings() {
        if let Ok(dir) = config::data_dir() {
            let _ = Command::new("explorer.exe").arg(dir).spawn();
        }
    }

    pub fn run() -> Result<(), String> {
        let dir = config::data_dir()?;
        fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

        // Same lock as rd-agent watch: only one scheduler can run per config.
        let lock = OpenOptions::new().create(true).read(true).write(true)
            .open(dir.join("watch.lock")).map_err(|e| e.to_string())?;
        lock.try_lock_exclusive().map_err(|_| "RD가 이미 실행 중입니다")?;

        let (tx, rx) = mpsc::channel();
        ACTIONS.set(tx.clone()).map_err(|_| "RD 채널 생성 실패")?;
        let work_dir = dir.clone();
        let worker = thread::spawn(move || worker(&work_dir, rx));

        let result = unsafe { message_loop() };

        // Let an in-flight Git operation finish before the program exits.
        let _ = tx.send(Action::Quit);
        let _ = worker.join();
        let _ = FileExt::unlock(&lock);
        result
    }

    unsafe fn message_loop() -> Result<(), String> {
        let instance = GetModuleHandleW(std::ptr::null());
        if instance.is_null() { return Err("Windows 모듈 확인 실패".into()); }
        let cls = WNDCLASSW {
            lpfnWndProc: Some(window_proc),
            hInstance: instance,
            lpszClassName: w!("RDTrayMessageWindow"),
            ..Default::default()
        };
        if RegisterClassW(&cls) == 0 { return Err("트레이 윈도우 클래스 등록 실패".into()); }
        let hwnd = CreateWindowExW(
            0, w!("RDTrayMessageWindow"), w!("RD"), 0,
            0, 0, 0, 0, HWND_MESSAGE, std::ptr::null_mut(), instance, std::ptr::null(),
        );
        if hwnd.is_null() { return Err("트레이 윈도우 생성 실패".into()); }
        let mut nid: NOTIFYICONDATAW = std::mem::zeroed();
        nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        nid.hWnd = hwnd;
        nid.uID = 1;
        nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
        nid.uCallbackMessage = TRAY_CALLBACK;
        nid.hIcon = LoadIconW(std::ptr::null_mut(), IDI_APPLICATION);
        let tooltip: Vec<u16> = "RD — RepoDock".encode_utf16().collect();
        nid.szTip[..tooltip.len()].copy_from_slice(&tooltip);
        if Shell_NotifyIconW(NIM_ADD, &nid) == 0 {
            DestroyWindow(hwnd);
            return Err("Windows 시스템 트레이 등록 실패".into());
        }

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, std::ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        Shell_NotifyIconW(NIM_DELETE, &nid);
        Ok(())
    }
}

#[cfg(windows)]
fn main() {
    if let Err(error) = app::run() {
        // Windows-subsystem process has no console, so persist startup failures.
        if let Ok(dir) = rd_agent::config::data_dir() {
            let _ = std::fs::create_dir_all(&dir);
            let _ = std::fs::write(dir.join("tray-error.txt"), error);
        }
    }
}
