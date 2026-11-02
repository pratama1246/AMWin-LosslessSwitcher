#![allow(unsafe_op_in_unsafe_fn)]

use std::ffi::c_void;
use windows::core::{HRESULT, PCWSTR, GUID, Interface};
use windows::Win32::Media::Audio::{WAVEFORMATEX, WAVEFORMATEXTENSIBLE, WAVEFORMATEXTENSIBLE_0};
use windows::Win32::System::Com::CLSCTX_ALL;
use crate::audio::AudioFormat;

pub const CLSID_POLICY_CONFIG_CLIENT: GUID = GUID::from_u128(0x870AF99C_171D_4F9E_AF0D_E63DF40C2BC9);
pub const CLSID_POLICY_CONFIG_CLIENT_ALT: GUID = GUID::from_u128(0xab27072e_0920_4af6_bb50_38d5474378f4);
pub const IID_IPOLICY_CONFIG: GUID = GUID::from_u128(0xF8679F50_850A_41CF_9C72_430F290290C8);

pub const WAVE_FORMAT_EXTENSIBLE: u16 = 0xFFFE;
pub const KSDATAFORMAT_SUBTYPE_PCM: GUID = GUID::from_u128(0x00000001_0000_0010_8000_00aa00389b71);
pub const KSDATAFORMAT_SUBTYPE_IEEE_FLOAT: GUID = GUID::from_u128(0x00000003_0000_0010_8000_00aa00389b71);

#[repr(C)]
pub struct IPolicyConfigVtbl {
    pub query_interface: usize,
    pub add_ref: usize,
    pub release: unsafe extern "system" fn(*mut c_void) -> u32,
    pub get_mix_format: unsafe extern "system" fn(*mut c_void, PCWSTR, *mut *mut WAVEFORMATEX) -> HRESULT,
    pub get_device_format: unsafe extern "system" fn(*mut c_void, PCWSTR, i32, *mut *mut WAVEFORMATEX) -> HRESULT,
    pub reset_device_format: unsafe extern "system" fn(*mut c_void, PCWSTR) -> HRESULT,
    pub set_device_format: unsafe extern "system" fn(*mut c_void, PCWSTR, *const WAVEFORMATEX, *const WAVEFORMATEX) -> HRESULT,
    pub get_processing_period: usize,
    pub set_processing_period: usize,
    pub get_share_mode: usize,
    pub set_share_mode: usize,
    pub get_property_value: usize,
    pub set_property_value: usize,
    pub set_default_endpoint: unsafe extern "system" fn(*mut c_void, PCWSTR, i32) -> HRESULT,
    pub set_endpoint_visibility: usize,
}

pub struct PolicyConfig {
    ptr: *mut *const IPolicyConfigVtbl,
}

unsafe impl Send for PolicyConfig {}
unsafe impl Sync for PolicyConfig {}

impl PolicyConfig {
    pub fn new() -> windows::core::Result<Self> {
        unsafe {
            let hr = windows::Win32::System::Com::CoCreateInstance(
                &CLSID_POLICY_CONFIG_CLIENT,
                None,
                CLSCTX_ALL,
            );
            match hr {
                Ok(unk) => {
                    let mut policy_ptr: *mut c_void = std::ptr::null_mut();
                    let unk: windows::core::IUnknown = unk;
                    unk.query(&IID_IPOLICY_CONFIG, &mut policy_ptr).ok()?;
                    Ok(Self {
                        ptr: policy_ptr as *mut *const IPolicyConfigVtbl,
                    })
                }
                Err(_) => {
                    let unk: windows::core::IUnknown = windows::Win32::System::Com::CoCreateInstance(
                        &CLSID_POLICY_CONFIG_CLIENT_ALT,
                        None,
                        CLSCTX_ALL,
                    )?;
                    let mut policy_ptr: *mut c_void = std::ptr::null_mut();
                    unk.query(&IID_IPOLICY_CONFIG, &mut policy_ptr).ok()?;
                    Ok(Self {
                        ptr: policy_ptr as *mut *const IPolicyConfigVtbl,
                    })
                }
            }
        }
    }

    pub fn set_device_format(&self, endpoint_id: &str, format: &AudioFormat) -> windows::core::Result<()> {
        let wide_id: Vec<u16> = endpoint_id.encode_utf16().chain(std::iter::once(0)).collect();
        let pcwstr = PCWSTR::from_raw(wide_id.as_ptr());

        let ep_format = create_endpoint_format(format);
        let mix_format = create_mix_format(format.sample_rate);

        let p_ep = &ep_format as *const WAVEFORMATEXTENSIBLE as *const WAVEFORMATEX;
        let p_mix = &mix_format as *const WAVEFORMATEXTENSIBLE as *const WAVEFORMATEX;

        unsafe {
            let vtbl = *self.ptr;
            let hr = ((*vtbl).set_device_format)(self.ptr as *mut c_void, pcwstr, p_ep, p_mix);
            hr.ok()
        }
    }

    #[allow(dead_code)]
    pub fn set_device_format_clamped(&self, endpoint_id: &str, target: &AudioFormat) -> windows::core::Result<AudioFormat> {
        let mut candidates = crate::audio::wasapi::WasapiManager::new()
            .map(|w| w.supported_candidates(endpoint_id, target))
            .unwrap_or_default();
        if candidates.is_empty() {
            candidates.push(*target);
        }
        let mut failed_rates = Vec::new();
        for c in candidates {
            if failed_rates.contains(&c.sample_rate) {
                continue;
            }
            if self.set_device_format(endpoint_id, &c).is_ok() {
                return Ok(c);
            } else {
                failed_rates.push(c.sample_rate);
                std::thread::sleep(std::time::Duration::from_millis(30));
            }
        }
        Err(windows::core::Error::from_hresult(windows::core::HRESULT(0x88890008_u32 as i32)))
    }



    pub fn get_device_format(&self, endpoint_id: &str) -> windows::core::Result<AudioFormat> {
        let wide_id: Vec<u16> = endpoint_id.encode_utf16().chain(std::iter::once(0)).collect();
        let pcwstr = PCWSTR::from_raw(wide_id.as_ptr());

        unsafe {
            let vtbl = *self.ptr;
            let mut dev_fmt: *mut WAVEFORMATEX = std::ptr::null_mut();
            let hr = ((*vtbl).get_device_format)(self.ptr as *mut c_void, pcwstr, 0, &mut dev_fmt);
            hr.ok()?;

            if dev_fmt.is_null() {
                return Ok(AudioFormat::new(44100, 16));
            }

            let f = *dev_fmt;
            let mut bits = f.wBitsPerSample;
            if f.wFormatTag == WAVE_FORMAT_EXTENSIBLE && f.cbSize >= 22 {
                let ext = &*(dev_fmt as *const WAVEFORMATEXTENSIBLE);
                let valid_bits = ext.Samples.wValidBitsPerSample;
                if valid_bits > 0 {
                    bits = valid_bits;
                }
            }
            let rate = f.nSamplesPerSec;

            windows::Win32::System::Com::CoTaskMemFree(Some(dev_fmt as *const _));
            Ok(AudioFormat::new(rate, bits))
        }
    }
}

impl Drop for PolicyConfig {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            unsafe {
                let vtbl = *self.ptr;
                ((*vtbl).release)(self.ptr as *mut c_void);
            }
        }
    }
}

pub(crate) fn create_endpoint_format(format: &AudioFormat) -> WAVEFORMATEXTENSIBLE {
    let (container_bits, valid_bits) = if format.bit_depth <= 16 {
        (16, 16)
    } else if format.bit_depth >= 32 {
        (32, 32)
    } else {
        (32, 24)
    };

    let block_align = (format.channels * container_bits) / 8;
    let avg_bytes_per_sec = format.sample_rate * block_align as u32;

    WAVEFORMATEXTENSIBLE {
        Format: WAVEFORMATEX {
            wFormatTag: WAVE_FORMAT_EXTENSIBLE,
            nChannels: format.channels,
            nSamplesPerSec: format.sample_rate,
            nAvgBytesPerSec: avg_bytes_per_sec,
            nBlockAlign: block_align,
            wBitsPerSample: container_bits,
            cbSize: 22,
        },
        Samples: WAVEFORMATEXTENSIBLE_0 {
            wValidBitsPerSample: valid_bits,
        },
        dwChannelMask: 0x00000003,
        SubFormat: KSDATAFORMAT_SUBTYPE_PCM,
    }
}

fn create_mix_format(sample_rate: u32) -> WAVEFORMATEXTENSIBLE {
    let block_align = (2 * 32) / 8;
    let avg_bytes_per_sec = sample_rate * block_align as u32;

    WAVEFORMATEXTENSIBLE {
        Format: WAVEFORMATEX {
            wFormatTag: WAVE_FORMAT_EXTENSIBLE,
            nChannels: 2,
            nSamplesPerSec: sample_rate,
            nAvgBytesPerSec: avg_bytes_per_sec,
            nBlockAlign: block_align,
            wBitsPerSample: 32,
            cbSize: 22,
        },
        Samples: WAVEFORMATEXTENSIBLE_0 {
            wValidBitsPerSample: 32,
        },
        dwChannelMask: 0x00000003,
        SubFormat: KSDATAFORMAT_SUBTYPE_IEEE_FLOAT,
    }
}
