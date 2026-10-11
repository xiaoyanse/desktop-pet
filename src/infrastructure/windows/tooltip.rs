use super::native::wide;
use windows_sys::Win32::{
    Foundation::*,
    System::LibraryLoader::GetModuleHandleW,
    UI::{Controls::*, WindowsAndMessaging::*},
};

pub struct Tooltip {
    hwnd: HWND,
    owner: HWND,
    text: Vec<u16>,
    enabled: bool,
}
impl Tooltip {
    pub fn new(owner: HWND) -> Option<Self> {
        unsafe {
            let controls = INITCOMMONCONTROLSEX {
                dwSize: std::mem::size_of::<INITCOMMONCONTROLSEX>() as u32,
                dwICC: ICC_WIN95_CLASSES,
            };
            InitCommonControlsEx(&controls);
            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW,
                wide("tooltips_class32").as_ptr(),
                std::ptr::null(),
                WS_POPUP | TTS_ALWAYSTIP | TTS_NOPREFIX,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                CW_USEDEFAULT,
                owner,
                std::ptr::null_mut(),
                GetModuleHandleW(std::ptr::null()),
                std::ptr::null(),
            );
            if hwnd.is_null() {
                return None;
            }
            let mut result = Self {
                hwnd,
                owner,
                text: wide("双击返回正常视图；右键打开菜单"),
                enabled: true,
            };
            let mut info = result.info();
            info.uFlags = TTF_IDISHWND | TTF_SUBCLASS;
            if SendMessageW(
                hwnd,
                TTM_ADDTOOLW,
                0,
                (&info as *const TTTOOLINFOW) as isize,
            ) == 0
            {
                DestroyWindow(hwnd);
                return None;
            }
            SendMessageW(hwnd, TTM_SETMAXTIPWIDTH, 0, 360);
            SendMessageW(hwnd, TTM_SETDELAYTIME, TTDT_INITIAL as usize, 350);
            SendMessageW(hwnd, TTM_SETDELAYTIME, TTDT_AUTOPOP as usize, 15000);
            result.activate(false);
            Some(result)
        }
    }
    fn info(&mut self) -> TTTOOLINFOW {
        TTTOOLINFOW {
            cbSize: std::mem::size_of::<TTTOOLINFOW>() as u32,
            hwnd: self.owner,
            uId: self.owner as usize,
            lpszText: self.text.as_mut_ptr(),
            ..Default::default()
        }
    }
    pub fn update(&mut self, text: &str) {
        let next = wide(text);
        if self.text == next {
            return;
        }
        self.text = next;
        let info = self.info();
        unsafe {
            SendMessageW(
                self.hwnd,
                TTM_UPDATETIPTEXTW,
                0,
                (&info as *const TTTOOLINFOW) as isize,
            );
        }
    }
    pub fn activate(&mut self, enabled: bool) {
        if self.enabled == enabled {
            return;
        }
        self.enabled = enabled;
        unsafe {
            SendMessageW(self.hwnd, TTM_ACTIVATE, enabled as usize, 0);
        }
    }
}
impl Drop for Tooltip {
    fn drop(&mut self) {
        unsafe {
            DestroyWindow(self.hwnd);
        }
    }
}
