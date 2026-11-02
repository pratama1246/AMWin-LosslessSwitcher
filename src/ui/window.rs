#![allow(unsafe_op_in_unsafe_fn)]

use std::sync::{Arc, Mutex};
use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM, COLORREF, RECT};
use windows::Win32::Graphics::Gdi::{
    CreateFontW, DeleteObject, HFONT, FW_NORMAL, FW_BOLD, UpdateWindow, HBITMAP, HBRUSH,
    GetDC, ReleaseDC, CreateCompatibleDC, DeleteDC, CreateCompatibleBitmap,
    SelectObject, CreateSolidBrush, FillRect, CreatePen, Rectangle,
    SetBkMode, SetTextColor, DrawTextW, PS_SOLID, TRANSPARENT,
    DT_CENTER, DT_VCENTER, DT_SINGLELINE,
    InvalidateRect, MapWindowPoints,
};
use windows::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, RegOpenKeyExW, RegQueryValueExW, RegSetValueExW, RegDeleteValueW,
    RegCloseKey, KEY_READ, KEY_WRITE, REG_SZ,
};
use windows::Win32::UI::Controls::{
    InitCommonControlsEx, INITCOMMONCONTROLSEX, INITCOMMONCONTROLSEX_ICC, ICC_BAR_CLASSES, ICC_STANDARD_CLASSES,
};
use windows::Win32::UI::Shell::{
    Shell_NotifyIconW, NOTIFYICONDATAW, NOTIFY_ICON_DATA_FLAGS, NOTIFY_ICON_INFOTIP_FLAGS,
    NIM_ADD, NIM_DELETE, NIF_ICON, NIF_MESSAGE, NIF_TIP, ShellExecuteW,
};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, LoadCursorW, LoadIconW,
    PostQuitMessage, RegisterClassExW, SendMessageW, SetTimer, ShowWindow, TranslateMessage,
    CreatePopupMenu, AppendMenuW, TrackPopupMenu, DestroyMenu, SetForegroundWindow, DestroyWindow,
    SetWindowPos, HWND_TOPMOST, HWND_NOTOPMOST, SWP_NOMOVE, SWP_NOSIZE, WS_CLIPCHILDREN,
    WM_SIZE, BM_GETCHECK, BS_AUTOCHECKBOX,
    CS_HREDRAW, CS_VREDRAW, CW_USEDEFAULT, HICON, HMENU, IDC_ARROW, IDI_APPLICATION,
    MSG, SW_HIDE, SW_RESTORE, SW_SHOW, SW_SHOWNORMAL, TPM_BOTTOMALIGN, TPM_LEFTALIGN,
    WINDOW_EX_STYLE, WNDCLASSEXW, WS_CHILD, WS_GROUP, WS_OVERLAPPED, WS_CAPTION,
    WS_SYSMENU, WS_MINIMIZEBOX, WS_VISIBLE, WM_CLOSE, WM_COMMAND, WM_CREATE,
    WM_DESTROY, WM_HSCROLL, WM_RBUTTONUP, WM_LBUTTONDBLCLK, WM_SETFONT,
    WM_SYSCOMMAND, WM_TIMER, WM_USER, WM_CTLCOLORSTATIC, WM_CTLCOLORBTN, WM_CTLCOLORDLG,
    MF_STRING, MF_CHECKED, MF_UNCHECKED,
    MF_SEPARATOR, BM_SETCHECK, CB_ADDSTRING, CB_GETCURSEL, CB_SETCURSEL,
    CB_RESETCONTENT, WM_SETICON, SWP_NOZORDER, SWP_NOACTIVATE, WM_GETFONT,
};
use crate::audio::AudioFormat;
use crate::audio::wasapi::{AudioDeviceInfo, WasapiManager};
use crate::coordinator::{Coordinator, SwitcherMode};

const CBS_DROPDOWNLIST: u32 = 0x0003;
const SS_BITMAP: u32 = 0x0000000E;
const SS_CENTERIMAGE: u32 = 0x00000200;
const SS_NOPREFIX: u32 = 0x00000080;
const STM_SETIMAGE: u32 = 0x0172;
const IMAGE_BITMAP: usize = 0;
const LOGO_PNG: &[u8] = include_bytes!("../../assets/logo.png");

const WM_TRAY_ICON: u32 = WM_USER + 1;
const TIMER_STATUS_ID: usize = 1001;

const ID_TAB_CONTROL: isize = 200;
const ID_DEVICE_COMBO: isize = 201;
const ID_RADIO_SEAMLESS: isize = 202;
const ID_RADIO_PURIST: isize = 203;
const ID_DELAY_TRACKBAR: isize = 204;
const ID_BTN_MINIMIZE: isize = 205;
const ID_BTN_REFRESH: isize = 206;
const ID_ARTWORK: isize = 207;
const ID_BTN_CLEAR_CACHE: isize = 208;
const ID_BTN_CLEAR_CACHE_ABOUT: isize = 209;
const ID_CHK_ALWAYS_ON_TOP: isize = 210;
const ID_CHK_STARTUP: isize = 211;
const ID_BTN_GITHUB: isize = 212;

const ID_TRAY_OPEN: usize = 301;
const ID_TRAY_SEAMLESS: usize = 302;
const ID_TRAY_PURIST: usize = 303;
const ID_TRAY_EXIT: usize = 304;
const ID_TRAY_ALWAYS_ON_TOP: usize = 305;
const ID_TRAY_STARTUP: usize = 306;

const TBM_SETRANGE: u32 = WM_USER + 6;
const TBM_SETPOS: u32 = WM_USER + 5;
const TBM_GETPOS: u32 = WM_USER;

const TCM_FIRST: u32 = 0x1300;
const TCM_INSERTITEMW: u32 = TCM_FIRST + 62;
const TCM_GETCURSEL: u32 = TCM_FIRST + 11;
const TCIF_TEXT: u32 = 0x0001;

const WM_NOTIFY: u32 = 0x004E;
const TCN_FIRST: u32 = 0u32.wrapping_sub(550);
const TCN_SELCHANGE: u32 = TCN_FIRST.wrapping_sub(1);

const NIF_INFO: u32 = 0x00000010;
const NIIF_INFO: u32 = 0x00000001;

#[repr(C)]
struct TCITEMW {
    mask: u32,
    dw_state: u32,
    dw_state_mask: u32,
    psz_text: *mut u16,
    cch_text_max: i32,
    i_image: i32,
    l_param: isize,
}

#[repr(C)]
struct NMHDR {
    hwnd_from: HWND,
    id_from: usize,
    code: u32,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn SetProcessDpiAwarenessContext(value: isize) -> i32;
    fn GetDpiForSystem() -> u32;
    fn GetDpiForWindow(hwnd: HWND) -> u32;
}

const WM_DPICHANGED: u32 = 0x02E0;

#[repr(C)]
struct GdiplusStartupInput {
    gdiplus_version: u32,
    debug_event_callback: usize,
    suppress_background_thread: i32,
    suppress_external_codecs: i32,
}

#[link(name = "gdiplus")]
unsafe extern "system" {
    fn GdiplusStartup(token: *mut usize, input: *const GdiplusStartupInput, output: *mut std::ffi::c_void) -> i32;
    fn GdiplusShutdown(token: usize);
    fn GdipLoadImageFromStream(stream: *mut std::ffi::c_void, image: *mut *mut std::ffi::c_void) -> i32;
    fn GdipCreateBitmapFromScan0(width: i32, height: i32, stride: i32, format: i32, scan0: *mut u8, bitmap: *mut *mut std::ffi::c_void) -> i32;
    fn GdipGetImageGraphicsContext(image: *mut std::ffi::c_void, graphics: *mut *mut std::ffi::c_void) -> i32;
    fn GdipSetInterpolationMode(graphics: *mut std::ffi::c_void, mode: i32) -> i32;
    fn GdipGetImageWidth(image: *mut std::ffi::c_void, width: *mut u32) -> i32;
    fn GdipGetImageHeight(image: *mut std::ffi::c_void, height: *mut u32) -> i32;
    fn GdipDrawImageRectRectI(graphics: *mut std::ffi::c_void, image: *mut std::ffi::c_void, dstx: i32, dsty: i32, dstwidth: i32, dstheight: i32, srcx: i32, srcy: i32, srcwidth: i32, srcheight: i32, srcUnit: i32, imageAttributes: *mut std::ffi::c_void, callback: usize, callbackData: usize) -> i32;
    fn GdipDeleteGraphics(graphics: *mut std::ffi::c_void) -> i32;
    fn GdipCreateHBITMAPFromBitmap(bitmap: *mut std::ffi::c_void, hbmReturn: *mut HBITMAP, background: u32) -> i32;
    fn GdipDisposeImage(image: *mut std::ffi::c_void) -> i32;
}

#[link(name = "shlwapi")]
unsafe extern "system" {
    fn SHCreateMemStream(pInit: *const u8, cbInit: u32) -> *mut std::ffi::c_void;
}

fn scale_val(val: i32, scale: f32) -> i32 {
    (val as f32 * scale).round() as i32
}

struct WindowState {
    main_hwnd: HWND,
    coordinator: Arc<Coordinator>,
    font: HFONT,
    font_bold: HFONT,
    font_title: HFONT,
    font_sub: HFONT,
    tab_ctrl: HWND,
    tab0_controls: Vec<HWND>,
    tab1_controls: Vec<HWND>,
    tab2_controls: Vec<HWND>,
    muted_labels: Vec<HWND>,
    title_labels: Vec<HWND>,
    combo_devices: HWND,
    radio_seamless: HWND,
    radio_purist: HWND,
    chk_always_on_top: HWND,
    chk_startup: HWND,
    trackbar_delay: HWND,
    label_delay: HWND,
    ctrl_artwork: HWND,
    ctrl_logo: HWND,
    label_track: HWND,
    label_formats: HWND,
    label_status: HWND,
    cached_devices: Vec<AudioDeviceInfo>,
    is_in_tray: bool,
    is_always_on_top: bool,
    current_art_hash: Option<u64>,
    current_hbitmap: Option<HBITMAP>,
    default_hbitmap: HBITMAP,
    logo_hbitmap: Option<HBITMAP>,
    artwork_size: i32,
    /// Every child control's rect in 96-DPI units (parent client space); rescaled from here on DPI change so rounding never drifts.
    base_rects: Vec<(HWND, RECT)>,
    last_notified_fmt: Option<AudioFormat>,
}

unsafe impl Send for WindowState {}
unsafe impl Sync for WindowState {}

static APP_STATE: Mutex<Option<WindowState>> = Mutex::new(None);
static INITIAL_COORDINATOR: Mutex<Option<Arc<Coordinator>>> = Mutex::new(None);
static GDIPLUS_TOKEN: Mutex<usize> = Mutex::new(0);
static BG_BRUSH: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);

unsafe fn get_bg_brush() -> HBRUSH {
    let raw = BG_BRUSH.load(std::sync::atomic::Ordering::Relaxed);
    if raw != 0 {
        HBRUSH(raw as *mut _)
    } else {
        let b = CreateSolidBrush(COLORREF(0x00F9F9F9));
        BG_BRUSH.store(b.0 as isize, std::sync::atomic::Ordering::Relaxed);
        b
    }
}

pub fn run_app() {
    unsafe {
        let _ = windows::Win32::System::Com::CoInitializeEx(
            None,
            windows::Win32::System::Com::COINIT_APARTMENTTHREADED,
        );

        // Enable Per-Monitor V2 DPI awareness for crystal-clear multi-DPI scaling
        let _ = SetProcessDpiAwarenessContext(-4);

        let mut gdi_tok: usize = 0;
        let gdi_in = GdiplusStartupInput {
            gdiplus_version: 1,
            debug_event_callback: 0,
            suppress_background_thread: 0,
            suppress_external_codecs: 0,
        };
        let _ = GdiplusStartup(&mut gdi_tok, &gdi_in, std::ptr::null_mut());
        if let Ok(mut guard) = GDIPLUS_TOKEN.lock() {
            *guard = gdi_tok;
        }

        let icex = INITCOMMONCONTROLSEX {
            dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
            dwICC: ICC_STANDARD_CLASSES | ICC_BAR_CLASSES | INITCOMMONCONTROLSEX_ICC(0x00000008),
        };
        let _ = InitCommonControlsEx(&icex);

        let hinstance = windows::Win32::System::LibraryLoader::GetModuleHandleW(None).unwrap();
        let class_name = w!("LosslessSwitcherX64Class");

        let app_icon = LoadIconW(hinstance, PCWSTR(1 as _))
            .unwrap_or_else(|_| LoadIconW(None, IDI_APPLICATION).unwrap_or(HICON(std::ptr::null_mut())));

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            cbClsExtra: 0,
            cbWndExtra: 0,
            hInstance: hinstance.into(),
            hIcon: app_icon,
            hCursor: LoadCursorW(None, IDC_ARROW).unwrap_or_default(),
            hbrBackground: CreateSolidBrush(COLORREF(0x00F9F9F9)),
            lpszMenuName: PCWSTR::null(),
            lpszClassName: class_name,
            hIconSm: app_icon,
        };

        RegisterClassExW(&wc);

        let saved_settings = crate::config::load_settings();
        let coordinator = Arc::new(Coordinator::new(saved_settings.mode, saved_settings.settling_delay_ms));
        if let Some(ref dev_id) = saved_settings.selected_device_id {
            coordinator.set_device_id(Some(dev_id.clone()));
        }
        coordinator.start();
        {
            let mut init_guard = INITIAL_COORDINATOR.lock().unwrap();
            *init_guard = Some(Arc::clone(&coordinator));
        }

        let dpi = GetDpiForSystem().max(96);
        let scale = dpi as f32 / 96.0;

        let start_in_tray = std::env::args().any(|a| a == "--minimized" || a == "--tray");

        let window_title = w!("LosslessSwitcher For Apple Music");
        let hwnd = CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            window_title,
            WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_CLIPCHILDREN,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            scale_val(505, scale),
            scale_val(420, scale),
            None,
            None,
            hinstance,
            None,
        ).unwrap();

        let _ = SendMessageW(hwnd, WM_SETICON, WPARAM(0), LPARAM(app_icon.0 as isize));
        let _ = SendMessageW(hwnd, WM_SETICON, WPARAM(1), LPARAM(app_icon.0 as isize));

        // Launched on a monitor whose DPI differs from the primary's (system) DPI: no WM_DPICHANGED fires, so scale now.
        let win_dpi = GetDpiForWindow(hwnd);
        if win_dpi != 0 && win_dpi != dpi {
            let s = win_dpi as f32 / 96.0;
            let _ = SetWindowPos(
                hwnd,
                HWND(std::ptr::null_mut()),
                0,
                0,
                scale_val(505, s),
                scale_val(420, s),
                SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
            );
            apply_dpi(win_dpi);
        }

        if !start_in_tray {
            let _ = ShowWindow(hwnd, SW_SHOW);
            let _ = UpdateWindow(hwnd);
        } else if let Ok(mut guard) = APP_STATE.lock() {
            if let Some(ref mut state) = *guard {
                state.is_in_tray = true;
            }
        }

        let mut msg = MSG::default();
        while GetMessageW(&mut msg, None, 0, 0).as_bool() {
            let _ = TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        coordinator.shutdown();

        if let Ok(guard) = GDIPLUS_TOKEN.lock() {
            if *guard != 0 {
                GdiplusShutdown(*guard);
            }
        }
        windows::Win32::System::Com::CoUninitialize();
    }
}

unsafe extern "system" fn collect_child(h: HWND, lparam: LPARAM) -> windows::Win32::Foundation::BOOL {
    (*(lparam.0 as *mut Vec<HWND>)).push(h);
    windows::Win32::Foundation::BOOL(1)
}

/// Re-lays-out the whole UI for a new monitor DPI: real fonts at the new pixel size (ClearType re-rasterizes, no bitmap
/// stretch), every child rect from its 96-DPI base, and artwork bitmaps at the new size.
/// Window calls run without APP_STATE locked because child resizes can re-enter wnd_proc.
unsafe fn apply_dpi(dpi: u32) {
    let scale = dpi as f32 / 96.0;
    let (rects, old_font, old_bold, old_title, old_sub) = match APP_STATE.lock() {
        Ok(g) => match g.as_ref() {
            Some(s) => (s.base_rects.clone(), s.font, s.font_bold, s.font_title, s.font_sub),
            None => return,
        },
        Err(_) => return,
    };

    let face = w!("Segoe UI");
    let font_title = CreateFontW(scale_val(-16, scale), 0, 0, 0, FW_BOLD.0 as i32, 0, 0, 0, 0, 0, 0, 0, 0, face);
    let font_bold = CreateFontW(scale_val(-13, scale), 0, 0, 0, FW_BOLD.0 as i32, 0, 0, 0, 0, 0, 0, 0, 0, face);
    let font = CreateFontW(scale_val(-12, scale), 0, 0, 0, FW_NORMAL.0 as i32, 0, 0, 0, 0, 0, 0, 0, 0, face);
    let font_sub = CreateFontW(scale_val(-11, scale), 0, 0, 0, FW_NORMAL.0 as i32, 0, 0, 0, 0, 0, 0, 0, 0, face);

    for (h, rc) in rects {
        let _ = SetWindowPos(
            h,
            HWND(std::ptr::null_mut()),
            scale_val(rc.left, scale),
            scale_val(rc.top, scale),
            scale_val(rc.right, scale) - scale_val(rc.left, scale),
            scale_val(rc.bottom, scale) - scale_val(rc.top, scale),
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
        let cur = SendMessageW(h, WM_GETFONT, WPARAM(0), LPARAM(0)).0;
        let f = if cur == old_title.0 as isize {
            font_title
        } else if cur == old_bold.0 as isize {
            font_bold
        } else if cur == old_sub.0 as isize {
            font_sub
        } else {
            font
        };
        SendMessageW(h, WM_SETFONT, WPARAM(f.0 as usize), LPARAM(1));
    }

    if let Ok(mut guard) = APP_STATE.lock() {
        if let Some(ref mut s) = *guard {
            s.font = font;
            s.font_bold = font_bold;
            s.font_title = font_title;
            s.font_sub = font_sub;
            s.artwork_size = scale_val(145, scale);
            let new_default = create_default_artwork_bitmap(s.artwork_size);
            SendMessageW(s.ctrl_artwork, STM_SETIMAGE, WPARAM(IMAGE_BITMAP), LPARAM(new_default.0 as isize));
            let _ = DeleteObject(windows::Win32::Graphics::Gdi::HGDIOBJ(s.default_hbitmap.0 as *mut _));
            s.default_hbitmap = new_default;
            if let Some(old) = s.current_hbitmap.take() {
                let _ = DeleteObject(windows::Win32::Graphics::Gdi::HGDIOBJ(old.0 as *mut _));
            }
            // Forces update_status_display to reload the real artwork at the new size on its next tick.
            s.current_art_hash = None;

            // Rescale About logo
            if let Some(new_logo) = load_image_from_bytes(LOGO_PNG, scale_val(48, scale)) {
                SendMessageW(s.ctrl_logo, STM_SETIMAGE, WPARAM(IMAGE_BITMAP), LPARAM(new_logo.0 as isize));
                if let Some(old_logo) = s.logo_hbitmap.replace(new_logo) {
                    let _ = DeleteObject(windows::Win32::Graphics::Gdi::HGDIOBJ(old_logo.0 as *mut _));
                }
            }
        }
    }
    let _ = DeleteObject(windows::Win32::Graphics::Gdi::HGDIOBJ(old_font.0 as *mut _));
    let _ = DeleteObject(windows::Win32::Graphics::Gdi::HGDIOBJ(old_bold.0 as *mut _));
    let _ = DeleteObject(windows::Win32::Graphics::Gdi::HGDIOBJ(old_title.0 as *mut _));
    let _ = DeleteObject(windows::Win32::Graphics::Gdi::HGDIOBJ(old_sub.0 as *mut _));
    let _ = InvalidateRect(None, None, true);
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_DPICHANGED => {
            let r = &*(lparam.0 as *const RECT);
            let _ = SetWindowPos(
                hwnd,
                HWND(std::ptr::null_mut()),
                r.left,
                r.top,
                r.right - r.left,
                r.bottom - r.top,
                SWP_NOZORDER | SWP_NOACTIVATE,
            );
            apply_dpi((wparam.0 & 0xFFFF) as u32);
            LRESULT(0)
        }
        WM_CREATE => {
            let coordinator = {
                let mut guard = INITIAL_COORDINATOR.lock().unwrap();
                guard.take().unwrap()
            };
            init_gui(hwnd, coordinator);
            SetTimer(hwnd, TIMER_STATUS_ID, 300, None);
            add_tray_icon(hwnd);
            LRESULT(0)
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLORBTN | WM_CTLCOLORDLG => {
            let hdc = windows::Win32::Graphics::Gdi::HDC(wparam.0 as _);
            SetBkMode(hdc, TRANSPARENT);
            let ctrl_hwnd = HWND(lparam.0 as *mut _);
            if let Ok(guard) = APP_STATE.try_lock() {
                if let Some(ref state) = *guard {
                    if state.muted_labels.contains(&ctrl_hwnd) {
                        SetTextColor(hdc, COLORREF(0x006E6E6E));
                    } else if state.title_labels.contains(&ctrl_hwnd) {
                        SetTextColor(hdc, COLORREF(0x00111111));
                    }
                }
            }
            LRESULT(get_bg_brush().0 as isize)
        }
        WM_TIMER => {
            if wparam.0 == TIMER_STATUS_ID {
                update_status_display();
            }
            LRESULT(0)
        }
        WM_HSCROLL => {
            if let Ok(mut guard) = APP_STATE.lock() {
                if let Some(ref mut state) = *guard {
                    if lparam.0 == state.trackbar_delay.0 as isize {
                        let pos = SendMessageW(state.trackbar_delay, TBM_GETPOS, WPARAM(0), LPARAM(0)).0 as u64;
                        let text = format!("DAC Delay: {} ms", pos);
                        set_window_text(state.label_delay, &text);
                        state.coordinator.set_settling_delay(pos);
                    }
                }
            }
            save_current_settings();
            LRESULT(0)
        }
        WM_COMMAND => {
            let cmd_id = (wparam.0 & 0xFFFF) as isize;
            match cmd_id {
                ID_RADIO_SEAMLESS => {
                    if let Ok(mut guard) = APP_STATE.lock() {
                        if let Some(ref mut state) = *guard {
                            SendMessageW(state.radio_seamless, BM_SETCHECK, WPARAM(1), LPARAM(0));
                            SendMessageW(state.radio_purist, BM_SETCHECK, WPARAM(0), LPARAM(0));
                            state.coordinator.set_mode(SwitcherMode::Seamless);
                        }
                    }
                    save_current_settings();
                }
                ID_RADIO_PURIST => {
                    if let Ok(mut guard) = APP_STATE.lock() {
                        if let Some(ref mut state) = *guard {
                            SendMessageW(state.radio_seamless, BM_SETCHECK, WPARAM(0), LPARAM(0));
                            SendMessageW(state.radio_purist, BM_SETCHECK, WPARAM(1), LPARAM(0));
                            state.coordinator.set_mode(SwitcherMode::Purist);
                        }
                    }
                    save_current_settings();
                }
                ID_DEVICE_COMBO => {
                    let code = (wparam.0 >> 16) & 0xFFFF;
                    if code == 1 {
                        if let Ok(mut guard) = APP_STATE.lock() {
                            if let Some(ref mut state) = *guard {
                                let idx = SendMessageW(state.combo_devices, CB_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                                if idx < state.cached_devices.len() {
                                    let dev = &state.cached_devices[idx];
                                    state.coordinator.set_device_id(Some(dev.id.clone()));
                                }
                            }
                        }
                        save_current_settings();
                    }
                }
                ID_BTN_REFRESH => {
                    refresh_device_list();
                    save_current_settings();
                }
                ID_BTN_MINIMIZE => {
                    let _ = ShowWindow(hwnd, SW_HIDE);
                    if let Ok(mut guard) = APP_STATE.lock() {
                        if let Some(ref mut state) = *guard {
                            state.is_in_tray = true;
                        }
                    }
                }
                ID_CHK_ALWAYS_ON_TOP => {
                    if let Ok(mut guard) = APP_STATE.lock() {
                        if let Some(ref mut state) = *guard {
                            let is_checked = SendMessageW(state.chk_always_on_top, BM_GETCHECK, WPARAM(0), LPARAM(0)).0 != 0;
                            state.is_always_on_top = is_checked;
                            set_always_on_top(state.main_hwnd, is_checked);
                        }
                    }
                    save_current_settings();
                }
                ID_CHK_STARTUP => {
                    if let Ok(guard) = APP_STATE.lock() {
                        if let Some(ref state) = *guard {
                            let is_checked = SendMessageW(state.chk_startup, BM_GETCHECK, WPARAM(0), LPARAM(0)).0 != 0;
                            set_startup_enabled(is_checked);
                        }
                    }
                }
                ID_BTN_CLEAR_CACHE | ID_BTN_CLEAR_CACHE_ABOUT => {
                    let dir = crate::config::get_app_dir();
                    let _ = std::fs::remove_file(dir.join("catalog_cache.json"));
                    let art_dir = dir.join("artworks");
                    let _ = std::fs::remove_dir_all(&art_dir);
                    if let Ok(mut guard) = APP_STATE.lock() {
                        if let Some(ref mut state) = *guard {
                            state.current_art_hash = None;
                            if let Some(old_hbm) = state.current_hbitmap.take() {
                                let _ = DeleteObject(windows::Win32::Graphics::Gdi::HGDIOBJ(old_hbm.0 as *mut _));
                            }
                            SendMessageW(
                                state.ctrl_artwork,
                                STM_SETIMAGE,
                                WPARAM(IMAGE_BITMAP),
                                LPARAM(state.default_hbitmap.0 as isize),
                            );
                            set_window_text(state.label_status, "Status: Cache cleared! Next track will refresh.");
                        }
                    }
                }
                ID_BTN_GITHUB => {
                    let _ = unsafe {
                        ShellExecuteW(
                            HWND::default(),
                            w!("open"),
                            w!("https://github.com/pratama1246/AMWin-LosslessSwitcher"),
                            PCWSTR::null(),
                            PCWSTR::null(),
                            SW_SHOWNORMAL,
                        )
                    };
                }
                _ => {
                    let menu_id = wparam.0;
                    match menu_id {
                        ID_TRAY_OPEN => {
                            let _ = ShowWindow(hwnd, SW_RESTORE);
                            let _ = SetForegroundWindow(hwnd);
                            if let Ok(mut guard) = APP_STATE.lock() {
                                if let Some(ref mut state) = *guard {
                                    state.is_in_tray = false;
                                }
                            }
                        }
                        ID_TRAY_SEAMLESS => {
                            if let Ok(mut guard) = APP_STATE.lock() {
                                if let Some(ref mut state) = *guard {
                                    SendMessageW(state.radio_seamless, BM_SETCHECK, WPARAM(1), LPARAM(0));
                                    SendMessageW(state.radio_purist, BM_SETCHECK, WPARAM(0), LPARAM(0));
                                    state.coordinator.set_mode(SwitcherMode::Seamless);
                                }
                            }
                            save_current_settings();
                        }
                        ID_TRAY_PURIST => {
                            if let Ok(mut guard) = APP_STATE.lock() {
                                if let Some(ref mut state) = *guard {
                                    SendMessageW(state.radio_seamless, BM_SETCHECK, WPARAM(0), LPARAM(0));
                                    SendMessageW(state.radio_purist, BM_SETCHECK, WPARAM(1), LPARAM(0));
                                    state.coordinator.set_mode(SwitcherMode::Purist);
                                }
                            }
                            save_current_settings();
                        }
                        ID_TRAY_ALWAYS_ON_TOP => {
                            if let Ok(mut guard) = APP_STATE.lock() {
                                if let Some(ref mut state) = *guard {
                                    state.is_always_on_top = !state.is_always_on_top;
                                    let check_val = if state.is_always_on_top { 1 } else { 0 };
                                    SendMessageW(state.chk_always_on_top, BM_SETCHECK, WPARAM(check_val), LPARAM(0));
                                    set_always_on_top(state.main_hwnd, state.is_always_on_top);
                                }
                            }
                            save_current_settings();
                        }
                        ID_TRAY_STARTUP => {
                            if let Ok(guard) = APP_STATE.lock() {
                                if let Some(ref state) = *guard {
                                    let new_val = !is_startup_enabled();
                                    set_startup_enabled(new_val);
                                    let check_val = if new_val { 1 } else { 0 };
                                    SendMessageW(state.chk_startup, BM_SETCHECK, WPARAM(check_val), LPARAM(0));
                                }
                            }
                        }
                        ID_TRAY_EXIT => {
                            let _ = DestroyWindow(hwnd);
                        }
                        _ => {}
                    }
                }
            }
            LRESULT(0)
        }
        WM_NOTIFY => {
            let p_nmhdr = lparam.0 as *const NMHDR;
            if !p_nmhdr.is_null() {
                let code = (*p_nmhdr).code;
                if code == TCN_SELCHANGE {
                    let controls = {
                        if let Ok(guard) = APP_STATE.lock() {
                            if let Some(ref state) = *guard {
                                let sel = SendMessageW(state.tab_ctrl, TCM_GETCURSEL, WPARAM(0), LPARAM(0)).0 as usize;
                                Some((sel, state.tab0_controls.clone(), state.tab1_controls.clone(), state.tab2_controls.clone()))
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    };

                    if let Some((sel, tab0, tab1, tab2)) = controls {
                        let show_tab0 = if sel == 0 { SW_SHOW } else { SW_HIDE };
                        let show_tab1 = if sel == 1 { SW_SHOW } else { SW_HIDE };
                        let show_tab2 = if sel == 2 { SW_SHOW } else { SW_HIDE };

                        for ctrl in tab0 {
                            let _ = ShowWindow(ctrl, show_tab0);
                        }
                        for ctrl in tab1 {
                            let _ = ShowWindow(ctrl, show_tab1);
                        }
                        for ctrl in tab2 {
                            let _ = ShowWindow(ctrl, show_tab2);
                        }
                        let _ = InvalidateRect(hwnd, None, true);
                        let _ = UpdateWindow(hwnd);
                    }
                }
            }
            LRESULT(0)
        }
        WM_SIZE => {
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_SYSCOMMAND => {
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_CLOSE => {
            let _ = ShowWindow(hwnd, SW_HIDE);
            if let Ok(mut guard) = APP_STATE.lock() {
                if let Some(ref mut state) = *guard {
                    state.is_in_tray = true;
                }
            }
            LRESULT(0)
        }
        WM_TRAY_ICON => {
            let event = (lparam.0 & 0xFFFF) as u32;
            match event {
                WM_LBUTTONDBLCLK => {
                    let _ = ShowWindow(hwnd, SW_RESTORE);
                    let _ = SetForegroundWindow(hwnd);
                    if let Ok(mut guard) = APP_STATE.lock() {
                        if let Some(ref mut state) = *guard {
                            state.is_in_tray = false;
                        }
                    }
                }
                WM_RBUTTONUP => {
                    show_tray_context_menu(hwnd);
                }
                _ => {}
            }
            LRESULT(0)
        }
        WM_DESTROY => {
            remove_tray_icon(hwnd);
            if let Ok(guard) = APP_STATE.lock() {
                if let Some(ref state) = *guard {
                    state.coordinator.shutdown();
                }
            }
            let raw = BG_BRUSH.swap(0, std::sync::atomic::Ordering::Relaxed);
            if raw != 0 {
                let _ = DeleteObject(windows::Win32::Graphics::Gdi::HGDIOBJ(raw as *mut _));
            }
            if let Ok(mut guard) = APP_STATE.lock() {
                if let Some(ref mut state) = *guard {
                    let _ = DeleteObject(windows::Win32::Graphics::Gdi::HGDIOBJ(state.font.0 as *mut _));
                    let _ = DeleteObject(windows::Win32::Graphics::Gdi::HGDIOBJ(state.font_bold.0 as *mut _));
                    let _ = DeleteObject(windows::Win32::Graphics::Gdi::HGDIOBJ(state.font_title.0 as *mut _));
                    let _ = DeleteObject(windows::Win32::Graphics::Gdi::HGDIOBJ(state.font_sub.0 as *mut _));
                    let _ = DeleteObject(windows::Win32::Graphics::Gdi::HGDIOBJ(state.default_hbitmap.0 as *mut _));
                    if let Some(hbm) = state.current_hbitmap.take() {
                        let _ = DeleteObject(windows::Win32::Graphics::Gdi::HGDIOBJ(hbm.0 as *mut _));
                    }
                    if let Some(hbm) = state.logo_hbitmap.take() {
                        let _ = DeleteObject(windows::Win32::Graphics::Gdi::HGDIOBJ(hbm.0 as *mut _));
                    }
                }
            }
            PostQuitMessage(0);
            LRESULT(0)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn init_gui(parent: HWND, coordinator: Arc<Coordinator>) {
    let hinstance = windows::Win32::System::LibraryLoader::GetModuleHandleW(None).unwrap();

    let dpi = GetDpiForSystem().max(96);
    let scale = dpi as f32 / 96.0;

    let font_name = w!("Segoe UI");
    let font_title = CreateFontW(
        scale_val(-16, scale), 0, 0, 0, FW_BOLD.0 as i32, 0, 0, 0,
        0, 0, 0, 0, 0, font_name,
    );
    let font_bold = CreateFontW(
        scale_val(-13, scale), 0, 0, 0, FW_BOLD.0 as i32, 0, 0, 0,
        0, 0, 0, 0, 0, font_name,
    );
    let font = CreateFontW(
        scale_val(-12, scale), 0, 0, 0, FW_NORMAL.0 as i32, 0, 0, 0,
        0, 0, 0, 0, 0, font_name,
    );
    let font_sub = CreateFontW(
        scale_val(-11, scale), 0, 0, 0, FW_NORMAL.0 as i32, 0, 0, 0,
        0, 0, 0, 0, 0, font_name,
    );

    // Native Tab Control (WC_TABCONTROLW) - Placed with comfortable margins
    let tab_ctrl = CreateWindowExW(
        WINDOW_EX_STYLE(0),
        w!("SysTabControl32"),
        w!(""),
        WS_CHILD | WS_VISIBLE | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(0),
        scale_val(12, scale),
        scale_val(10, scale),
        scale_val(465, scale),
        scale_val(355, scale),
        parent,
        HMENU(ID_TAB_CONTROL as *mut _),
        hinstance,
        None,
    ).unwrap();
    SendMessageW(tab_ctrl, WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));

    // Tab 0: "Now Playing"
    let mut tab0_text: Vec<u16> = "Now Playing".encode_utf16().chain(std::iter::once(0)).collect();
    let mut tie0 = TCITEMW {
        mask: TCIF_TEXT,
        dw_state: 0,
        dw_state_mask: 0,
        psz_text: tab0_text.as_mut_ptr(),
        cch_text_max: tab0_text.len() as i32,
        i_image: -1,
        l_param: 0,
    };
    SendMessageW(tab_ctrl, TCM_INSERTITEMW, WPARAM(0), LPARAM(&mut tie0 as *mut _ as isize));

    // Tab 1: "Audio Settings"
    let mut tab1_text: Vec<u16> = "Audio Settings".encode_utf16().chain(std::iter::once(0)).collect();
    let mut tie1 = TCITEMW {
        mask: TCIF_TEXT,
        dw_state: 0,
        dw_state_mask: 0,
        psz_text: tab1_text.as_mut_ptr(),
        cch_text_max: tab1_text.len() as i32,
        i_image: -1,
        l_param: 0,
    };
    SendMessageW(tab_ctrl, TCM_INSERTITEMW, WPARAM(1), LPARAM(&mut tie1 as *mut _ as isize));

    // Tab 2: "About"
    let mut tab2_text: Vec<u16> = "About".encode_utf16().chain(std::iter::once(0)).collect();
    let mut tie2 = TCITEMW {
        mask: TCIF_TEXT,
        dw_state: 0,
        dw_state_mask: 0,
        psz_text: tab2_text.as_mut_ptr(),
        cch_text_max: tab2_text.len() as i32,
        i_image: -1,
        l_param: 0,
    };
    SendMessageW(tab_ctrl, TCM_INSERTITEMW, WPARAM(2), LPARAM(&mut tie2 as *mut _ as isize));

    // ==========================================
    // TAB 0 CONTROLS (Now Playing - Visible by default)
    // ==========================================
    let artwork_size = scale_val(145, scale);
    let ctrl_artwork = create_ctrl(
        w!("STATIC"), w!(""),
        WS_CHILD | WS_VISIBLE | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(SS_BITMAP | SS_CENTERIMAGE),
        scale_val(28, scale), scale_val(50, scale), artwork_size, artwork_size, parent, Some(ID_ARTWORK), hinstance,
    );

    let default_hbitmap = create_default_artwork_bitmap(artwork_size);
    SendMessageW(
        ctrl_artwork,
        STM_SETIMAGE,
        WPARAM(IMAGE_BITMAP),
        LPARAM(default_hbitmap.0 as isize),
    );

    let info_x = scale_val(190, scale);
    let info_w = scale_val(270, scale);

    let label_track = create_ctrl(
        w!("STATIC"), w!("Track: Waiting for Apple Music..."),
        WS_CHILD | WS_VISIBLE | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(SS_NOPREFIX),
        info_x, scale_val(50, scale), info_w, scale_val(40, scale), parent, None, hinstance,
    );
    SendMessageW(label_track, WM_SETFONT, WPARAM(font_bold.0 as usize), LPARAM(1));

    let label_formats = create_ctrl(
        w!("STATIC"), w!("Source: -\nDAC: -"),
        WS_CHILD | WS_VISIBLE | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(SS_NOPREFIX),
        info_x, scale_val(96, scale), info_w, scale_val(36, scale), parent, None, hinstance,
    );
    SendMessageW(label_formats, WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));

    let label_status = create_ctrl(
        w!("STATIC"), w!("Status: Ready & waiting for playback..."),
        WS_CHILD | WS_VISIBLE | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(SS_NOPREFIX),
        info_x, scale_val(138, scale), info_w, scale_val(52, scale), parent, None, hinstance,
    );
    SendMessageW(label_status, WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));

    // Hide to Tray button placed neatly at bottom-left directly under album artwork
    let btn_min = create_ctrl(
        w!("BUTTON"), w!("Hide to Tray"),
        WS_CHILD | WS_VISIBLE | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(windows::Win32::UI::WindowsAndMessaging::BS_PUSHBUTTON as u32),
        scale_val(28, scale), scale_val(212, scale), artwork_size, scale_val(32, scale), parent, Some(ID_BTN_MINIMIZE), hinstance,
    );
    SendMessageW(btn_min, WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));

    let tab0_controls = vec![
        ctrl_artwork,
        label_track,
        label_formats,
        label_status,
        btn_min,
    ];

    // ==========================================
    // TAB 1 CONTROLS (Audio & Settings - Hidden by default)
    // ==========================================
    let grp_device = create_ctrl(
        w!("BUTTON"), w!(" Output Device "),
        WS_CHILD | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(windows::Win32::UI::WindowsAndMessaging::BS_GROUPBOX as u32),
        scale_val(24, scale), scale_val(44, scale), scale_val(441, scale), scale_val(60, scale), parent, None, hinstance,
    );
    SendMessageW(grp_device, WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));

    let combo_devices = create_ctrl(
        w!("COMBOBOX"), w!(""),
        WS_CHILD | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(CBS_DROPDOWNLIST) | WS_GROUP,
        scale_val(38, scale), scale_val(64, scale), scale_val(325, scale), scale_val(180, scale), parent, Some(ID_DEVICE_COMBO), hinstance,
    );
    SendMessageW(combo_devices, WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));

    let btn_refresh = create_ctrl(
        w!("BUTTON"), w!("Refresh"),
        WS_CHILD | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(windows::Win32::UI::WindowsAndMessaging::BS_PUSHBUTTON as u32),
        scale_val(373, scale), scale_val(63, scale), scale_val(80, scale), scale_val(26, scale), parent, Some(ID_BTN_REFRESH), hinstance,
    );
    SendMessageW(btn_refresh, WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));

    let grp_mode = create_ctrl(
        w!("BUTTON"), w!(" Switcher Mode "),
        WS_CHILD | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(windows::Win32::UI::WindowsAndMessaging::BS_GROUPBOX as u32),
        scale_val(24, scale), scale_val(110, scale), scale_val(441, scale), scale_val(68, scale), parent, None, hinstance,
    );
    SendMessageW(grp_mode, WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));

    let saved_settings = crate::config::load_settings();
    let (chk_seamless, chk_purist) = if saved_settings.mode == SwitcherMode::Purist {
        (0, 1)
    } else {
        (1, 0)
    };

    let radio_seamless = create_ctrl(
        w!("BUTTON"), w!("Seamless (Recommended - switches sample rate without audio pop)"),
        WS_CHILD | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(windows::Win32::UI::WindowsAndMessaging::BS_AUTORADIOBUTTON as u32) | WS_GROUP,
        scale_val(38, scale), scale_val(128, scale), scale_val(415, scale), scale_val(20, scale), parent, Some(ID_RADIO_SEAMLESS), hinstance,
    );
    SendMessageW(radio_seamless, WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));
    SendMessageW(radio_seamless, BM_SETCHECK, WPARAM(chk_seamless), LPARAM(0));

    let radio_purist = create_ctrl(
        w!("BUTTON"), w!("Purist (Bit-perfect - strictly matches track bit depth)"),
        WS_CHILD | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(windows::Win32::UI::WindowsAndMessaging::BS_AUTORADIOBUTTON as u32),
        scale_val(38, scale), scale_val(150, scale), scale_val(415, scale), scale_val(20, scale), parent, Some(ID_RADIO_PURIST), hinstance,
    );
    SendMessageW(radio_purist, WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));
    SendMessageW(radio_purist, BM_SETCHECK, WPARAM(chk_purist), LPARAM(0));

    let grp_pref = create_ctrl(
        w!("BUTTON"), w!(" Preferences "),
        WS_CHILD | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(windows::Win32::UI::WindowsAndMessaging::BS_GROUPBOX as u32),
        scale_val(24, scale), scale_val(184, scale), scale_val(441, scale), scale_val(68, scale), parent, None, hinstance,
    );
    SendMessageW(grp_pref, WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));

    let chk_always_on_top = create_ctrl(
        w!("BUTTON"), w!("Always on Top"),
        WS_CHILD | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
        scale_val(38, scale), scale_val(202, scale), scale_val(415, scale), scale_val(20, scale), parent, Some(ID_CHK_ALWAYS_ON_TOP), hinstance,
    );
    SendMessageW(chk_always_on_top, WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));
    if saved_settings.is_always_on_top {
        SendMessageW(chk_always_on_top, BM_SETCHECK, WPARAM(1), LPARAM(0));
        set_always_on_top(parent, true);
    }

    let chk_startup = create_ctrl(
        w!("BUTTON"), w!("Run on Windows Startup (Minimizes to System Tray)"),
        WS_CHILD | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(BS_AUTOCHECKBOX as u32),
        scale_val(38, scale), scale_val(224, scale), scale_val(415, scale), scale_val(20, scale), parent, Some(ID_CHK_STARTUP), hinstance,
    );
    SendMessageW(chk_startup, WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));
    if is_startup_enabled() {
        SendMessageW(chk_startup, BM_SETCHECK, WPARAM(1), LPARAM(0));
    }

    let grp_delay = create_ctrl(
        w!("BUTTON"), w!(" Settling Delay && Cache "),
        WS_CHILD | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(windows::Win32::UI::WindowsAndMessaging::BS_GROUPBOX as u32),
        scale_val(24, scale), scale_val(258, scale), scale_val(441, scale), scale_val(80, scale), parent, None, hinstance,
    );
    SendMessageW(grp_delay, WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));

    let delay_text = format!("DAC Delay: {} ms", saved_settings.settling_delay_ms);
    let wide_delay: Vec<u16> = delay_text.encode_utf16().chain(std::iter::once(0)).collect();
    let label_delay = create_ctrl(
        w!("STATIC"), PCWSTR::from_raw(wide_delay.as_ptr()),
        WS_CHILD | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(SS_NOPREFIX),
        scale_val(38, scale), scale_val(276, scale), scale_val(160, scale), scale_val(18, scale), parent, None, hinstance,
    );
    SendMessageW(label_delay, WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));

    let trackbar_delay = create_ctrl(
        w!("msctls_trackbar32"), w!(""),
        WS_CHILD,
        scale_val(38, scale), scale_val(298, scale), scale_val(270, scale), scale_val(24, scale), parent, Some(ID_DELAY_TRACKBAR), hinstance,
    );
    SendMessageW(trackbar_delay, TBM_SETRANGE, WPARAM(1), LPARAM(((800 << 16) | 50) as isize));
    SendMessageW(trackbar_delay, TBM_SETPOS, WPARAM(1), LPARAM(saved_settings.settling_delay_ms as isize));

    let btn_clear_cache = create_ctrl(
        w!("BUTTON"), w!("Clear Cache"),
        WS_CHILD | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(windows::Win32::UI::WindowsAndMessaging::BS_PUSHBUTTON as u32),
        scale_val(325, scale), scale_val(294, scale), scale_val(128, scale), scale_val(28, scale), parent, Some(ID_BTN_CLEAR_CACHE), hinstance,
    );
    SendMessageW(btn_clear_cache, WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));

    let tab1_controls = vec![
        grp_device,
        combo_devices,
        btn_refresh,
        grp_mode,
        radio_seamless,
        radio_purist,
        grp_pref,
        chk_always_on_top,
        chk_startup,
        grp_delay,
        label_delay,
        trackbar_delay,
        btn_clear_cache,
    ];

    // ==========================================
    // TAB 2 CONTROLS (About - Hidden by default)
    // ==========================================
    // 1. Judul
    let label_app_title = create_ctrl(
        w!("STATIC"), w!("LosslessSwitcher For Apple Music"),
        WS_CHILD | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(SS_NOPREFIX),
        scale_val(40, scale), scale_val(62, scale), scale_val(408, scale), scale_val(22, scale), parent, None, hinstance,
    );
    SendMessageW(label_app_title, WM_SETFONT, WPARAM(font_title.0 as usize), LPARAM(1));

    // 2. Version
    let label_app_version = create_ctrl(
        w!("STATIC"), w!("Version 0.2.0 (Native 64-bit Windows)"),
        WS_CHILD | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(SS_NOPREFIX),
        scale_val(40, scale), scale_val(86, scale), scale_val(408, scale), scale_val(16, scale), parent, None, hinstance,
    );
    SendMessageW(label_app_version, WM_SETFONT, WPARAM(font_sub.0 as usize), LPARAM(1));

    // 3. Desc
    let label_app_desc = create_ctrl(
        w!("STATIC"), w!("Automatically synchronizes DAC sample rate with Apple Music on Windows for bit-perfect audio playback."),
        WS_CHILD | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(SS_NOPREFIX),
        scale_val(40, scale), scale_val(106, scale), scale_val(408, scale), scale_val(32, scale), parent, None, hinstance,
    );
    SendMessageW(label_app_desc, WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));

    // 4. LOGO di tengah (centered horizontally at X = 221)
    let logo_size = scale_val(48, scale);
    let logo_hbm = load_image_from_bytes(LOGO_PNG, logo_size);
    let ctrl_logo = create_ctrl(
        w!("STATIC"), w!(""),
        WS_CHILD | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(SS_BITMAP),
        scale_val(221, scale), scale_val(146, scale), logo_size, logo_size, parent, None, hinstance,
    );
    if let Some(hbm) = logo_hbm {
        SendMessageW(ctrl_logo, STM_SETIMAGE, WPARAM(IMAGE_BITMAP), LPARAM(hbm.0 as isize));
    }

    // 5. Optimized For Apple Music For Windows
    let label_app_optimized = create_ctrl(
        w!("STATIC"), w!("Optimized for Apple Music for Windows"),
        WS_CHILD | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(SS_NOPREFIX),
        scale_val(40, scale), scale_val(202, scale), scale_val(408, scale), scale_val(16, scale), parent, None, hinstance,
    );
    SendMessageW(label_app_optimized, WM_SETFONT, WPARAM(font_sub.0 as usize), LPARAM(1));

    // 6. Not Affiliated with Apple inc
    let label_app_disclaimer = create_ctrl(
        w!("STATIC"), w!("Not affiliated with Apple Inc."),
        WS_CHILD | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(SS_NOPREFIX),
        scale_val(40, scale), scale_val(220, scale), scale_val(408, scale), scale_val(16, scale), parent, None, hinstance,
    );
    SendMessageW(label_app_disclaimer, WM_SETFONT, WPARAM(font_sub.0 as usize), LPARAM(1));

    // 7. Logo Copyright (simbol ©) + All rights reserved
    let label_app_copyright = create_ctrl(
        w!("STATIC"), w!("Developed by Pratama Putra © 2026. All rights reserved."),
        WS_CHILD | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(SS_NOPREFIX),
        scale_val(40, scale), scale_val(238, scale), scale_val(408, scale), scale_val(16, scale), parent, None, hinstance,
    );
    SendMessageW(label_app_copyright, WM_SETFONT, WPARAM(font_sub.0 as usize), LPARAM(1));

    // 8. GitHub button
    let btn_github = create_ctrl(
        w!("BUTTON"), w!("GitHub"),
        WS_CHILD | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(windows::Win32::UI::WindowsAndMessaging::BS_PUSHBUTTON as u32),
        scale_val(40, scale), scale_val(258, scale), scale_val(105, scale), scale_val(28, scale), parent, Some(ID_BTN_GITHUB), hinstance,
    );
    SendMessageW(btn_github, WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));

    // 9. Clear Cache button
    let btn_clear_cache_about = create_ctrl(
        w!("BUTTON"), w!("Clear Cache"),
        WS_CHILD | windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE(windows::Win32::UI::WindowsAndMessaging::BS_PUSHBUTTON as u32),
        scale_val(345, scale), scale_val(258, scale), scale_val(105, scale), scale_val(28, scale), parent, Some(ID_BTN_CLEAR_CACHE_ABOUT), hinstance,
    );
    SendMessageW(btn_clear_cache_about, WM_SETFONT, WPARAM(font.0 as usize), LPARAM(1));

    let tab2_controls = vec![
        label_app_title,
        label_app_version,
        label_app_desc,
        ctrl_logo,
        label_app_optimized,
        label_app_disclaimer,
        label_app_copyright,
        btn_github,
        btn_clear_cache_about,
    ];

    let muted_labels = vec![
        label_app_version,
        label_app_optimized,
        label_app_disclaimer,
        label_app_copyright,
    ];
    let title_labels = vec![
        label_app_title,
    ];

    let mut kids: Vec<HWND> = Vec::new();
    let _ = windows::Win32::UI::WindowsAndMessaging::EnumChildWindows(
        parent,
        Some(collect_child),
        LPARAM(&mut kids as *mut Vec<HWND> as isize),
    );
    let to_base = 96.0 / dpi as f32;
    let base_rects: Vec<(HWND, RECT)> = kids
        .into_iter()
        .filter_map(|h| {
            let mut rc = RECT::default();
            windows::Win32::UI::WindowsAndMessaging::GetWindowRect(h, &mut rc).ok()?;
            let mut p = [
                windows::Win32::Foundation::POINT { x: rc.left, y: rc.top },
                windows::Win32::Foundation::POINT { x: rc.right, y: rc.bottom },
            ];
            MapWindowPoints(HWND(std::ptr::null_mut()), parent, &mut p);
            Some((h, RECT {
                left: scale_val(p[0].x, to_base),
                top: scale_val(p[0].y, to_base),
                right: scale_val(p[1].x, to_base),
                bottom: scale_val(p[1].y, to_base),
            }))
        })
        .collect();

    {
        let mut guard = APP_STATE.lock().unwrap();
        *guard = Some(WindowState {
            main_hwnd: parent,
            coordinator,
            font,
            font_bold,
            font_title,
            font_sub,
            tab_ctrl,
            tab0_controls,
            tab1_controls,
            tab2_controls,
            muted_labels,
            title_labels,
            combo_devices,
            radio_seamless,
            radio_purist,
            chk_always_on_top,
            chk_startup,
            trackbar_delay,
            label_delay,
            ctrl_artwork,
            ctrl_logo,
            label_track,
            label_formats,
            label_status,
            cached_devices: Vec::new(),
            is_in_tray: false,
            is_always_on_top: saved_settings.is_always_on_top,
            current_art_hash: None,
            current_hbitmap: None,
            default_hbitmap,
            logo_hbitmap: logo_hbm,
            artwork_size,
            base_rects,
            last_notified_fmt: None,
        });
    }

    refresh_device_list();
}

unsafe fn create_ctrl(
    class: PCWSTR,
    text: PCWSTR,
    style: windows::Win32::UI::WindowsAndMessaging::WINDOW_STYLE,
    x: i32, y: i32, w: i32, h: i32,
    parent: HWND,
    id: Option<isize>,
    hinstance: windows::Win32::Foundation::HMODULE,
) -> HWND {
    let menu = match id {
        Some(val) => HMENU(val as *mut _),
        None => HMENU(std::ptr::null_mut()),
    };
    CreateWindowExW(
        WINDOW_EX_STYLE(0),
        class,
        text,
        style,
        x, y, w, h,
        parent,
        menu,
        hinstance,
        None,
    ).unwrap()
}

unsafe fn save_current_settings() {
    if let Ok(guard) = APP_STATE.lock() {
        if let Some(ref state) = *guard {
            let status = state.coordinator.get_status();
            let settings = crate::config::AppSettings {
                mode: status.mode,
                settling_delay_ms: status.settling_delay_ms,
                is_always_on_top: state.is_always_on_top,
                selected_device_id: state.coordinator.get_device_id(),
            };
            crate::config::save_settings(&settings);
        }
    }
}

unsafe fn refresh_device_list() {
    if let Ok(wasapi) = WasapiManager::new() {
        if let Ok(devices) = wasapi.list_render_devices() {
            if let Ok(mut guard) = APP_STATE.lock() {
                if let Some(ref mut state) = *guard {
                    SendMessageW(state.combo_devices, CB_RESETCONTENT, WPARAM(0), LPARAM(0));

                    let saved_dev_id = state.coordinator.get_device_id();
                    let mut select_idx = 0;
                    let mut matched_saved = false;

                    for (i, dev) in devices.iter().enumerate() {
                        let wide_name: Vec<u16> = dev.name.encode_utf16().chain(std::iter::once(0)).collect();
                        let pcwstr = PCWSTR::from_raw(wide_name.as_ptr());
                        SendMessageW(state.combo_devices, CB_ADDSTRING, WPARAM(0), LPARAM(pcwstr.as_ptr() as isize));

                        if let Some(ref target_id) = saved_dev_id {
                            if &dev.id == target_id {
                                select_idx = i;
                                matched_saved = true;
                            }
                        }

                        if !matched_saved && (dev.name.to_lowercase().contains("jadeaudio") 
                            || dev.name.to_lowercase().contains("ja11") 
                            || dev.is_default) {
                            select_idx = i;
                        }
                    }

                    if !devices.is_empty() {
                        SendMessageW(state.combo_devices, CB_SETCURSEL, WPARAM(select_idx), LPARAM(0));
                        state.coordinator.set_device_id(Some(devices[select_idx].id.clone()));
                    }

                    state.cached_devices = devices;
                }
            }
        }
    }
}

unsafe fn update_status_display() {
    if let Ok(mut guard) = APP_STATE.lock() {
        if let Some(ref mut state) = *guard {
            let status = state.coordinator.get_status();

            // Update artwork if in-memory bytes changed
            let new_art_hash = status.artwork_bytes.as_ref().map(|b| hash_bytes(b));
            if state.current_art_hash != new_art_hash {
                state.current_art_hash = new_art_hash;

                if let Some(old_hbm) = state.current_hbitmap.take() {
                    let _ = DeleteObject(windows::Win32::Graphics::Gdi::HGDIOBJ(old_hbm.0 as *mut _));
                }

                let hbm_to_display = if let Some(ref bytes) = status.artwork_bytes {
                    if let Some(hbm) = load_image_from_bytes(bytes, state.artwork_size) {
                        state.current_hbitmap = Some(hbm);
                        hbm
                    } else {
                        state.default_hbitmap
                    }
                } else {
                    state.default_hbitmap
                };

                SendMessageW(
                    state.ctrl_artwork,
                    STM_SETIMAGE,
                    WPARAM(IMAGE_BITMAP),
                    LPARAM(hbm_to_display.0 as isize),
                );
            }

            let track_text = match status.current_track {
                Some(ref t) => {
                    let clean_art = t.artist.split(['—', '–']).next().unwrap_or(&t.artist).trim();
                    let display_title = if t.title.chars().count() > 38 {
                        let tr: String = t.title.chars().take(35).collect();
                        format!("{}...", tr.trim_end())
                    } else {
                        t.title.clone()
                    };
                    let full = format!("Track: {} - {}", clean_art, display_title);
                    if full.chars().count() > 62 {
                        let tr: String = full.chars().take(59).collect();
                        format!("{}...", tr.trim_end())
                    } else {
                        full
                    }
                }
                None => "Track: Waiting for Apple Music...".to_string(),
            };
            set_window_text(state.label_track, &track_text);

            let target_str = if !status.is_lossless {
                "AAC (Lossy)".to_string()
            } else {
                status.target_format.map(|f| f.display_str()).unwrap_or_else(|| "-".to_string())
            };
            let dev_str = status.current_device_format.map(|f| f.display_str()).unwrap_or_else(|| "-".to_string());
            let fmt_text = format!("Source: {}\nDAC: {}", target_str, dev_str);
            set_window_text(state.label_formats, &fmt_text);

            let status_msg = if status.status_message.chars().count() > 56 {
                let tr: String = status.status_message.chars().take(53).collect();
                format!("{}...", tr.trim_end())
            } else {
                status.status_message.clone()
            };
            let status_text = format!("Status: {}", status_msg);
            set_window_text(state.label_status, &status_text);

            if let Some(cur_fmt) = status.current_device_format {
                if !status.is_syncing && state.last_notified_fmt != Some(cur_fmt) {
                    if state.last_notified_fmt.is_some() {
                        let note_msg = if !status.is_lossless {
                            format!("Output: {} (AAC Lossy)", cur_fmt.display_str())
                        } else if status.status_message.contains("(Clamped)") {
                            format!("Output: {} (Clamped)", cur_fmt.display_str())
                        } else {
                            format!("Output: {}", cur_fmt.display_str())
                        };
                        show_native_notification(
                            state.main_hwnd,
                            "DAC Format Switched",
                            &note_msg,
                        );
                    }
                    state.last_notified_fmt = Some(cur_fmt);
                }
            }
        }
    }
}

unsafe fn show_native_notification(hwnd: HWND, title: &str, message: &str) {
    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        uFlags: NOTIFY_ICON_DATA_FLAGS(NIF_INFO),
        dwInfoFlags: NOTIFY_ICON_INFOTIP_FLAGS(NIIF_INFO),
        ..Default::default()
    };
    let w_title: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let w_msg: Vec<u16> = message.encode_utf16().chain(std::iter::once(0)).collect();
    let title_len = w_title.len().min(63);
    let msg_len = w_msg.len().min(255);
    std::ptr::copy_nonoverlapping(w_title.as_ptr(), nid.szInfoTitle.as_mut_ptr(), title_len);
    std::ptr::copy_nonoverlapping(w_msg.as_ptr(), nid.szInfo.as_mut_ptr(), msg_len);
    let _ = Shell_NotifyIconW(windows::Win32::UI::Shell::NIM_MODIFY, &nid);
}

fn hash_bytes(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf29ce484222325;
    for &b in bytes {
        h ^= b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    h
}

unsafe fn load_image_from_bytes(bytes: &[u8], target_size: i32) -> Option<HBITMAP> {
    if bytes.is_empty() {
        return None;
    }
    let p_stream = SHCreateMemStream(bytes.as_ptr(), bytes.len() as u32);
    if p_stream.is_null() {
        return None;
    }

    let mut image: *mut std::ffi::c_void = std::ptr::null_mut();
    let load_res = GdipLoadImageFromStream(p_stream, &mut image);

    // COM Release stream
    let vtbl = *(p_stream as *mut *mut usize);
    let release_fn: unsafe extern "system" fn(*mut std::ffi::c_void) -> u32 =
        std::mem::transmute(*vtbl.add(2));
    release_fn(p_stream);

    if load_res != 0 || image.is_null() {
        return None;
    }

    let mut src_w: u32 = 0;
    let mut src_h: u32 = 0;
    let _ = GdipGetImageWidth(image, &mut src_w);
    let _ = GdipGetImageHeight(image, &mut src_h);

    if src_w == 0 || src_h == 0 {
        GdipDisposeImage(image);
        return None;
    }

    // High Quality Bicubic scaling to physical pixel size
    let mut dst_img: *mut std::ffi::c_void = std::ptr::null_mut();
    if GdipCreateBitmapFromScan0(target_size, target_size, 0, 0x26200A, std::ptr::null_mut(), &mut dst_img) != 0 || dst_img.is_null() {
        let mut fallback_hbm = HBITMAP(std::ptr::null_mut());
        let _ = GdipCreateHBITMAPFromBitmap(image, &mut fallback_hbm, 0);
        GdipDisposeImage(image);
        return if !fallback_hbm.0.is_null() { Some(fallback_hbm) } else { None };
    }

    let mut gfx: *mut std::ffi::c_void = std::ptr::null_mut();
    if GdipGetImageGraphicsContext(dst_img, &mut gfx) == 0 && !gfx.is_null() {
        // InterpolationModeHighQualityBicubic = 7
        let _ = GdipSetInterpolationMode(gfx, 7);
        // UnitPixel = 2
        let _ = GdipDrawImageRectRectI(gfx, image, 0, 0, target_size, target_size, 0, 0, src_w as i32, src_h as i32, 2, std::ptr::null_mut(), 0, 0);
        let _ = GdipDeleteGraphics(gfx);
    }

    let mut hbitmap = HBITMAP(std::ptr::null_mut());
    let status = GdipCreateHBITMAPFromBitmap(dst_img, &mut hbitmap, 0);
    GdipDisposeImage(dst_img);
    GdipDisposeImage(image);

    if status == 0 && !hbitmap.0.is_null() {
        Some(hbitmap)
    } else {
        None
    }
}

unsafe fn create_default_artwork_bitmap(size: i32) -> HBITMAP {
    let hdc_screen = GetDC(None);
    let hdc_mem = CreateCompatibleDC(hdc_screen);
    let hbm = CreateCompatibleBitmap(hdc_screen, size, size);

    let old_bmp = SelectObject(hdc_mem, hbm);

    let brush_bg = CreateSolidBrush(COLORREF(0x002F2825));
    let rect = RECT { left: 0, top: 0, right: size, bottom: size };
    FillRect(hdc_mem, &rect, brush_bg);
    let _ = DeleteObject(brush_bg);

    let pen = CreatePen(PS_SOLID, 1, COLORREF(0x004E443F));
    let old_pen = SelectObject(hdc_mem, pen);
    let _ = Rectangle(hdc_mem, 0, 0, size, size);
    SelectObject(hdc_mem, old_pen);
    let _ = DeleteObject(pen);

    SetBkMode(hdc_mem, TRANSPARENT);
    SetTextColor(hdc_mem, COLORREF(0x00908880));
    let mut symbol = [0x266Au16];
    let top_y = (size as f32 * 0.3) as i32;
    let bot_y = (size as f32 * 0.7) as i32;
    let mut text_rect = RECT { left: 0, top: top_y, right: size, bottom: bot_y };
    DrawTextW(hdc_mem, &mut symbol, &mut text_rect, DT_CENTER | DT_VCENTER | DT_SINGLELINE);

    SelectObject(hdc_mem, old_bmp);
    let _ = DeleteDC(hdc_mem);
    ReleaseDC(None, hdc_screen);

    hbm
}

unsafe fn set_window_text(hwnd: HWND, text: &str) {
    let len = windows::Win32::UI::WindowsAndMessaging::GetWindowTextLengthW(hwnd);
    if len > 0 {
        let mut buf = vec![0u16; (len + 1) as usize];
        let read = windows::Win32::UI::WindowsAndMessaging::GetWindowTextW(hwnd, &mut buf);
        if read as usize == len as usize {
            if let Ok(cur) = String::from_utf16(&buf[..len as usize]) {
                if cur == text {
                    return;
                }
            }
        }
    } else if text.is_empty() {
        return;
    }

    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowTextW(hwnd, PCWSTR::from_raw(wide.as_ptr()));
    if let Ok(parent) = windows::Win32::UI::WindowsAndMessaging::GetParent(hwnd) {
        if !parent.0.is_null() {
            let mut rect = RECT::default();
            let _ = windows::Win32::UI::WindowsAndMessaging::GetWindowRect(hwnd, &mut rect);
            let mut points = [
                windows::Win32::Foundation::POINT { x: rect.left, y: rect.top },
                windows::Win32::Foundation::POINT { x: rect.right, y: rect.bottom },
            ];
            MapWindowPoints(HWND(std::ptr::null_mut()), parent, &mut points);
            let parent_rect = RECT {
                left: points[0].x,
                top: points[0].y,
                right: points[1].x,
                bottom: points[1].y,
            };
            let _ = InvalidateRect(parent, Some(&parent_rect), true);
        }
    }
    let _ = InvalidateRect(hwnd, None, true);
}

unsafe fn add_tray_icon(hwnd: HWND) {
    let hinstance = windows::Win32::System::LibraryLoader::GetModuleHandleW(None).unwrap();
    let app_icon = LoadIconW(hinstance, PCWSTR(1 as _))
        .unwrap_or_else(|_| LoadIconW(None, IDI_APPLICATION).unwrap_or(HICON(std::ptr::null_mut())));

    let mut nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        uFlags: NIF_ICON | NIF_MESSAGE | NIF_TIP,
        uCallbackMessage: WM_TRAY_ICON,
        hIcon: app_icon,
        ..Default::default()
    };
    let tip = w!("LosslessSwitcher For Apple Music");
    let tip_slice = tip.as_wide();
    let copy_len = tip_slice.len().min(nid.szTip.len() - 1);
    std::ptr::copy_nonoverlapping(tip_slice.as_ptr(), nid.szTip.as_mut_ptr(), copy_len);
    let _ = Shell_NotifyIconW(NIM_ADD, &nid);
}

unsafe fn remove_tray_icon(hwnd: HWND) {
    let nid = NOTIFYICONDATAW {
        cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32,
        hWnd: hwnd,
        uID: 1,
        ..Default::default()
    };
    let _ = Shell_NotifyIconW(NIM_DELETE, &nid);
}

unsafe fn set_always_on_top(hwnd: HWND, enable: bool) {
    let flag = if enable {
        HWND_TOPMOST
    } else {
        HWND_NOTOPMOST
    };
    let _ = SetWindowPos(hwnd, flag, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE);
}

unsafe fn is_startup_enabled() -> bool {
    let mut hkey = HKEY::default();
    let subkey = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
    if RegOpenKeyExW(HKEY_CURRENT_USER, subkey, 0, KEY_READ, &mut hkey).is_err() {
        return false;
    }
    let res = RegQueryValueExW(hkey, w!("LosslessSwitcherAM"), None, None, None, None);
    let _ = RegCloseKey(hkey);
    res.is_ok()
}

unsafe fn set_startup_enabled(enable: bool) {
    let mut hkey = HKEY::default();
    let subkey = w!("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
    if RegOpenKeyExW(HKEY_CURRENT_USER, subkey, 0, KEY_WRITE, &mut hkey).is_err() {
        return;
    }
    if enable {
        if let Ok(exe_path) = std::env::current_exe() {
            let cmd = format!("\"{}\" --minimized", exe_path.display());
            let wide: Vec<u16> = cmd.encode_utf16().chain(std::iter::once(0)).collect();
            let slice: &[u8] = std::slice::from_raw_parts(
                wide.as_ptr() as *const u8,
                wide.len() * std::mem::size_of::<u16>(),
            );
            let _ = RegSetValueExW(hkey, w!("LosslessSwitcherAM"), 0, REG_SZ, Some(slice));
        }
    } else {
        let _ = RegDeleteValueW(hkey, w!("LosslessSwitcherAM"));
    }
    let _ = RegCloseKey(hkey);
}

unsafe fn show_tray_context_menu(hwnd: HWND) {
    let menu = CreatePopupMenu().unwrap();
    let (current_mode, is_top) = APP_STATE.lock().ok()
        .and_then(|guard| guard.as_ref().map(|s| (s.coordinator.get_status().mode, s.is_always_on_top)))
        .unwrap_or((SwitcherMode::Seamless, false));
    let is_startup = is_startup_enabled();

    let _ = AppendMenuW(menu, MF_STRING, ID_TRAY_OPEN, w!("Open LosslessSwitcher For Apple Music"));
    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());

    let top_flag = if is_top { MF_CHECKED } else { MF_UNCHECKED };
    let _ = AppendMenuW(menu, MF_STRING | top_flag, ID_TRAY_ALWAYS_ON_TOP, w!("Always on Top"));

    let start_flag = if is_startup { MF_CHECKED } else { MF_UNCHECKED };
    let _ = AppendMenuW(menu, MF_STRING | start_flag, ID_TRAY_STARTUP, w!("Run on Startup"));

    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());

    let seamless_flag = if current_mode == SwitcherMode::Seamless { MF_CHECKED } else { MF_UNCHECKED };
    let _ = AppendMenuW(menu, MF_STRING | seamless_flag, ID_TRAY_SEAMLESS, w!("Seamless Mode (Recommended)"));

    let purist_flag = if current_mode == SwitcherMode::Purist { MF_CHECKED } else { MF_UNCHECKED };
    let _ = AppendMenuW(menu, MF_STRING | purist_flag, ID_TRAY_PURIST, w!("Purist Mode (Bit-Perfect)"));

    let _ = AppendMenuW(menu, MF_SEPARATOR, 0, PCWSTR::null());
    let _ = AppendMenuW(menu, MF_STRING, ID_TRAY_EXIT, w!("Exit"));

    let mut cursor = windows::Win32::Foundation::POINT::default();
    let _ = windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut cursor);

    let _ = SetForegroundWindow(hwnd);
    let _ = TrackPopupMenu(menu, TPM_LEFTALIGN | TPM_BOTTOMALIGN, cursor.x, cursor.y, 0, hwnd, None);
    let _ = DestroyMenu(menu);
}
