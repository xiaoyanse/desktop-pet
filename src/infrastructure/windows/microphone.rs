use crate::{domain::meeting::MicrophoneStatus, interfaces::MicrophoneSource};
use windows::Win32::{
    Devices::FunctionDiscovery::PKEY_Device_FriendlyName,
    Media::Audio::{
        Endpoints::IAudioEndpointVolume, IMMDeviceEnumerator, MMDeviceEnumerator, eCapture,
        eCommunications,
    },
    System::Com::{
        CLSCTX_ALL, COINIT_MULTITHREADED, CoCreateInstance, CoInitializeEx, CoTaskMemFree,
        CoUninitialize, STGM_READ,
        StructuredStorage::{PROPVARIANT, PropVariantClear, PropVariantToStringAlloc},
    },
};
pub struct WindowsMicrophone;
struct Apartment;
impl Drop for Apartment {
    fn drop(&mut self) {
        unsafe {
            CoUninitialize();
        }
    }
}
struct Value(PROPVARIANT);
impl Drop for Value {
    fn drop(&mut self) {
        let _ = unsafe { PropVariantClear(&mut self.0) };
    }
}
struct Text(windows::core::PWSTR);
impl Drop for Text {
    fn drop(&mut self) {
        unsafe {
            CoTaskMemFree(Some(self.0.0.cast()));
        }
    }
}
impl MicrophoneSource for WindowsMicrophone {
    fn sample(&self) -> Result<MicrophoneStatus, String> {
        unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) }
            .ok()
            .map_err(|e| format!("无法初始化音频状态查询：{e}"))?;
        let _apartment = Apartment;
        let enumerator: IMMDeviceEnumerator =
            unsafe { CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL) }
                .map_err(|e| format!("无法枚举通信输入设备：{e}"))?;
        let device = unsafe { enumerator.GetDefaultAudioEndpoint(eCapture, eCommunications) }
            .map_err(|e| format!("没有可读取的默认通信麦克风：{e}"))?;
        let volume: IAudioEndpointVolume = unsafe { device.Activate(CLSCTX_ALL, None) }
            .map_err(|e| format!("麦克风控制状态不可读：{e}"))?;
        let muted = unsafe { volume.GetMute() }
            .map_err(|e| format!("麦克风静音状态不可读：{e}"))?
            .as_bool();
        let level = unsafe { volume.GetMasterVolumeLevelScalar() }
            .ok()
            .filter(|v| v.is_finite())
            .map(|v| (v.clamp(0.0, 1.0) * 100.0).round() as u8);
        let name = (|| -> windows::core::Result<String> {
            let properties = unsafe { device.OpenPropertyStore(STGM_READ) }?;
            let value = Value(unsafe { properties.GetValue(&PKEY_Device_FriendlyName) }?);
            let text = Text(unsafe { PropVariantToStringAlloc(&value.0) }?);
            let name = unsafe { text.0.to_string() }?;
            Ok(name.chars().take(256).collect())
        })()
        .unwrap_or_else(|_| "默认通信麦克风".into());
        Ok(MicrophoneStatus {
            name,
            muted,
            volume: level,
        })
    }
}
