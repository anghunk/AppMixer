use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use sysinfo::{Pid, System};
use windows::{
    core::*,
    Win32::Foundation::S_OK,
    Win32::Media::Audio::*,
    Win32::System::Com::*,
    Win32::Media::Audio::Endpoints::IAudioEndpointVolume,
    Win32::UI::Shell::SHLoadIndirectString,
};
use crate::icon;

#[derive(Debug, Serialize, Clone)]
pub struct AudioSession {
    pub pid: u32,
    pub name: String,
    pub volume: f32,
    pub muted: bool,
    pub icon: Option<String>,
}

pub struct AudioManager {
    sys: System,
    icon_cache: HashMap<PathBuf, String>,
}

impl AudioManager {
    pub fn new() -> Self {
        let mut sys = System::new_all();
        sys.refresh_all();
        Self { 
            sys,
            icon_cache: HashMap::new(),
        }
    }

    pub fn list_sessions(&mut self) -> Result<Vec<AudioSession>> {
        self.sys.refresh_processes();
        let mut sessions = Vec::new();

        unsafe {
            // Initialize COM library
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED).ok();

            let device_enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;

            let device: IMMDevice =
                device_enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;

            let session_manager: IAudioSessionManager2 =
                device.Activate(CLSCTX_ALL, None)?;

            let session_enumerator = session_manager.GetSessionEnumerator()?;
            let count = session_enumerator.GetCount()?;

            for i in 0..count {
                let session_control: IAudioSessionControl = session_enumerator.GetSession(i)?;
                let session_control2: IAudioSessionControl2 = session_control.cast()?;
                
                let pid = session_control2.GetProcessId()?;
                let is_system_sounds = session_control2.IsSystemSoundsSession() == S_OK;
                
                if pid == 0 && !is_system_sounds {
                    continue; // Skip invalid PIDs
                }

                let simple_volume: ISimpleAudioVolume = session_control.cast()?;
                let volume = simple_volume.GetMasterVolume()?;
                let muted = simple_volume.GetMute()?.as_bool();

                let (name, exe_path) = if is_system_sounds || pid == 0 {
                    let mut display_name = session_control.GetDisplayName()?.to_string().unwrap_or_default();
                    
                    if display_name.is_empty() {
                         display_name = "@%SystemRoot%\\System32\\AudioSrv.Dll,-202".to_string();
                    }

                    if display_name.starts_with('@') {
                         let mut buffer = [0u16; 512];
                         let display_name_h = HSTRING::from(&display_name);
                         let _ = SHLoadIndirectString(
                            PCWSTR(display_name_h.as_ptr()), 
                            &mut buffer, 
                            None
                         );
                         let localized_name = String::from_utf16_lossy(&buffer);
                         let clean_name = localized_name.trim_matches('\0').to_string();
                         (if clean_name.is_empty() { "系统声音".to_string() } else { clean_name }, None)
                    } else {
                        (display_name, None)
                    }
                } else {
                     if let Some(process) = self.sys.process(Pid::from(pid as usize)) {
                        (
                            process.name().to_string(),
                            process.exe().map(|p| p.to_path_buf())
                        )
                    } else {
                        (format!("Unknown Process ({})", pid), None)
                    }
                };

                // Get icon from cache or extract it
                let icon_base64 = if let Some(path) = exe_path {
                    if let Some(cached_icon) = self.icon_cache.get(path.as_path()) {
                        Some(cached_icon.clone())
                    } else {
                        if let Some(path_str) = path.to_str() {
                            match icon::extract_icon_to_base64(path_str) {
                                Ok(base64) => {
                                    self.icon_cache.insert(path.clone(), base64.clone());
                                    Some(base64)
                                }
                                Err(_) => None,
                            }
                        } else {
                            None
                        }
                    }
                } else {
                    None
                };

                sessions.push(AudioSession {
                    pid,
                    name,
                    volume,
                    muted,
                    icon: icon_base64,
                });
            }
        }

        Ok(sessions)
    }

    pub fn set_volume(&self, pid: u32, volume: f32) -> Result<()> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED).ok();

            let device_enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;

            let device: IMMDevice =
                device_enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;

            let session_manager: IAudioSessionManager2 =
                device.Activate(CLSCTX_ALL, None)?;

            let session_enumerator = session_manager.GetSessionEnumerator()?;
            let count = session_enumerator.GetCount()?;

            for i in 0..count {
                let session_control: IAudioSessionControl = session_enumerator.GetSession(i)?;
                let session_control2: IAudioSessionControl2 = session_control.cast()?;
                
                if session_control2.GetProcessId()? == pid {
                    let simple_volume: ISimpleAudioVolume = session_control.cast()?;
                    simple_volume.SetMasterVolume(volume, std::ptr::null())?;
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    pub fn set_mute(&self, pid: u32, muted: bool) -> Result<()> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED).ok();

            let device_enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;

            let device: IMMDevice =
                device_enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;

            let session_manager: IAudioSessionManager2 =
                device.Activate(CLSCTX_ALL, None)?;

            let session_enumerator = session_manager.GetSessionEnumerator()?;
            let count = session_enumerator.GetCount()?;

            for i in 0..count {
                let session_control: IAudioSessionControl = session_enumerator.GetSession(i)?;
                let session_control2: IAudioSessionControl2 = session_control.cast()?;
                
                if session_control2.GetProcessId()? == pid {
                    let simple_volume: ISimpleAudioVolume = session_control.cast()?;
                    simple_volume.SetMute(muted, std::ptr::null())?;
                    return Ok(());
                }
            }
        }
        Ok(())
    }

    // --- System Master Volume ---

    pub fn get_system_volume(&self) -> Result<f32> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED).ok();

            let device_enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;

            let device: IMMDevice =
                device_enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;

            let endpoint_volume: IAudioEndpointVolume =
                device.Activate(CLSCTX_ALL, None)?;

            let volume = endpoint_volume.GetMasterVolumeLevelScalar()?;
            Ok(volume)
        }
    }

    pub fn set_system_volume(&self, volume: f32) -> Result<()> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED).ok();

            let device_enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;

            let device: IMMDevice =
                device_enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;

            let endpoint_volume: IAudioEndpointVolume =
                device.Activate(CLSCTX_ALL, None)?;

            endpoint_volume.SetMasterVolumeLevelScalar(volume, std::ptr::null())?;
            Ok(())
        }
    }

    pub fn get_system_mute(&self) -> Result<bool> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED).ok();

            let device_enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;

            let device: IMMDevice =
                device_enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;

            let endpoint_volume: IAudioEndpointVolume =
                device.Activate(CLSCTX_ALL, None)?;

            let muted = endpoint_volume.GetMute()?.as_bool();
            Ok(muted)
        }
    }

    pub fn set_system_mute(&self, muted: bool) -> Result<()> {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_MULTITHREADED).ok();

            let device_enumerator: IMMDeviceEnumerator =
                CoCreateInstance(&MMDeviceEnumerator, None, CLSCTX_ALL)?;

            let device: IMMDevice =
                device_enumerator.GetDefaultAudioEndpoint(eRender, eMultimedia)?;

            let endpoint_volume: IAudioEndpointVolume =
                device.Activate(CLSCTX_ALL, None)?;

            endpoint_volume.SetMute(muted, std::ptr::null())?;
            Ok(())
        }
    }
}
