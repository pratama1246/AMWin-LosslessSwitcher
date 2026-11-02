#![allow(unsafe_op_in_unsafe_fn)]

use windows::core::{GUID, Result};
use windows::Win32::Media::Audio::{
    IMMDevice, IMMDeviceCollection, IMMDeviceEnumerator, MMDeviceEnumerator,
    eRender, eConsole, DEVICE_STATE_ACTIVE, IAudioClient, WAVEFORMATEX, WAVEFORMATEXTENSIBLE,
};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_ALL, STGM_READ};
use windows::Win32::UI::Shell::PropertiesSystem::{IPropertyStore, PROPERTYKEY};
use crate::audio::AudioFormat;
use crate::audio::policy_config::{create_endpoint_format, WAVE_FORMAT_EXTENSIBLE};

pub const PKEY_DEVICE_FRIENDLY_NAME: PROPERTYKEY = PROPERTYKEY {
    fmtid: GUID::from_u128(0xa45c254e_df1c_4efd_8020_67d146a850e0),
    pid: 14,
};

#[derive(Debug, Clone)]
pub struct AudioDeviceInfo {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

pub struct WasapiManager {
    enumerator: IMMDeviceEnumerator,
}

unsafe impl Send for WasapiManager {}
unsafe impl Sync for WasapiManager {}

impl WasapiManager {
    pub fn new() -> Result<Self> {
        let enumerator: IMMDeviceEnumerator = unsafe {
            CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?
        };
        Ok(Self { enumerator })
    }

    pub fn list_render_devices(&self) -> Result<Vec<AudioDeviceInfo>> {
        let mut devices = Vec::new();
        let default_id = self.get_default_device_id().unwrap_or_default();

        unsafe {
            let collection: IMMDeviceCollection = self.enumerator.EnumAudioEndpoints(
                eRender,
                DEVICE_STATE_ACTIVE,
            )?;
            let count = collection.GetCount()?;

            for i in 0..count {
                if let Ok(device) = collection.Item(i)
                    && let Ok(id_str) = get_device_id(&device) {
                        let name = get_device_friendly_name(&device)
                            .unwrap_or_else(|_| "Unknown Audio Device".to_string());
                        let is_default = id_str == default_id;
                        devices.push(AudioDeviceInfo {
                            id: id_str,
                            name,
                            is_default,
                        });
                    }
            }
        }
        Ok(devices)
    }

    pub fn get_default_device_id(&self) -> Result<String> {
        unsafe {
            let device: IMMDevice = self.enumerator.GetDefaultAudioEndpoint(eRender, eConsole)?;
            get_device_id(&device)
        }
    }

    pub fn get_current_format(&self, endpoint_id: &str) -> Result<AudioFormat> {
        unsafe {
            let wide_id: Vec<u16> = endpoint_id.encode_utf16().chain(std::iter::once(0)).collect();
            let device = self.enumerator.GetDevice(windows::core::PCWSTR::from_raw(wide_id.as_ptr()))?;
            let client: IAudioClient = device.Activate(CLSCTX_ALL, None)?;
            let mix_format_ptr = client.GetMixFormat()?;
            let format = parse_wave_format(mix_format_ptr);
            windows::Win32::System::Com::CoTaskMemFree(Some(mix_format_ptr as *const _));
            Ok(format)
        }
    }

    pub fn probe_device_ready(&self, endpoint_id: &str) -> bool {
        unsafe {
            let wide_id: Vec<u16> = endpoint_id.encode_utf16().chain(std::iter::once(0)).collect();
            let device = match self.enumerator.GetDevice(windows::core::PCWSTR::from_raw(wide_id.as_ptr())) {
                Ok(d) => d,
                Err(_) => return false,
            };

            let client: IAudioClient = match device.Activate(CLSCTX_ALL, None) {
                Ok(c) => c,
                Err(_) => return false,
            };

            let mix_format = match client.GetMixFormat() {
                Ok(f) => f,
                Err(_) => return false,
            };

            let init_res = client.Initialize(
                windows::Win32::Media::Audio::AUDCLNT_SHAREMODE_SHARED,
                0,
                1000000,
                0,
                mix_format,
                None,
            );
            windows::Win32::System::Com::CoTaskMemFree(Some(mix_format as *const _));
            if init_res.is_err() {
                return false;
            }

            // Deeper probe: verify that the audio render client service is ready
            // (ensures APO filters and driver render pins have attached properly)
            client.GetService::<windows::Win32::Media::Audio::IAudioRenderClient>().is_ok()
        }
    }

    /// Passive driver query: does not change the device format or invalidate running streams.
    /// ponytail: exclusive-mode query reports hardware capability; returns false if another app holds the device exclusively
    pub fn is_format_supported(&self, endpoint_id: &str, format: &AudioFormat) -> bool {
        unsafe {
            let wide_id: Vec<u16> = endpoint_id.encode_utf16().chain(std::iter::once(0)).collect();
            let Ok(device) = self.enumerator.GetDevice(windows::core::PCWSTR::from_raw(wide_id.as_ptr())) else { return false };
            let Ok(client) = device.Activate::<IAudioClient>(CLSCTX_ALL, None) else { return false };
            let ep = create_endpoint_format(format);
            client
                .IsFormatSupported(
                    windows::Win32::Media::Audio::AUDCLNT_SHAREMODE_EXCLUSIVE,
                    &ep as *const WAVEFORMATEXTENSIBLE as *const WAVEFORMATEX,
                    None,
                )
                .0 == 0
        }
    }

    /// Formats at or below `target` the driver reports as supported (rate descending, then depth target/24/16).
    /// ponytail: this query over-reports on some drivers (Realtek claims 192k/24 it can't apply), so callers must still
    /// verify with SetDeviceFormat and learn a per-device ceiling; it only filters out what is impossible (e.g. 32-bit).
    pub fn supported_candidates(&self, endpoint_id: &str, target: &AudioFormat) -> Vec<AudioFormat> {
        let mut depths = vec![target.bit_depth];
        for d in [24, 16] {
            if !depths.contains(&d) {
                depths.push(d);
            }
        }
        let all: Vec<AudioFormat> = [192000u32, 176400, 96000, 88200, 48000, 44100]
            .into_iter()
            .filter(|&r| r <= target.sample_rate)
            .flat_map(|r| depths.clone().into_iter().map(move |d| AudioFormat::new(r, d)))
            .collect();
        let supported: Vec<AudioFormat> = all
            .iter()
            .copied()
            .filter(|c| self.is_format_supported(endpoint_id, c))
            .collect();
        if supported.is_empty() {
            all
        } else {
            supported
        }
    }

}

unsafe fn get_device_id(device: &IMMDevice) -> Result<String> {
    let raw_id = device.GetId()?;
    let id_str = raw_id.to_string().map_err(|_| windows::core::Error::empty())?;
    windows::Win32::System::Com::CoTaskMemFree(Some(raw_id.as_ptr() as *const _));
    Ok(id_str)
}

unsafe fn get_device_friendly_name(device: &IMMDevice) -> Result<String> {
    let store: IPropertyStore = device.OpenPropertyStore(STGM_READ)?;
    let prop = store.GetValue(&PKEY_DEVICE_FRIENDLY_NAME)?;
    let name = prop.to_string();
    if name.is_empty() {
        Ok("Audio Device".to_string())
    } else {
        Ok(name)
    }
}

fn parse_wave_format(fmt: *const WAVEFORMATEX) -> AudioFormat {
    if fmt.is_null() {
        return AudioFormat::new(44100, 16);
    }
    unsafe {
        let f = &*fmt;
        let mut bit_depth = f.wBitsPerSample;
        if f.wFormatTag == WAVE_FORMAT_EXTENSIBLE {
            let ext = &*(fmt as *const WAVEFORMATEXTENSIBLE);
            let valid_bits = ext.Samples.wValidBitsPerSample;
            if valid_bits > 0 {
                bit_depth = valid_bits;
            }
        }
        AudioFormat {
            sample_rate: f.nSamplesPerSec,
            bit_depth,
            channels: f.nChannels,
        }
    }
}
