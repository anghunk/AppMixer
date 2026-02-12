// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod audio;
mod icon;
use audio::AudioManager;
use std::sync::Mutex;
use tauri::State;

// Define a wrapper struct for the state
struct AudioState(Mutex<AudioManager>);

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("你好, {}! 欢迎使用 Tauri!", name)
}

#[tauri::command]
fn get_audio_sessions(state: State<'_, AudioState>) -> Result<Vec<audio::AudioSession>, String> {
    let mut manager = state.0.lock().map_err(|_| "Failed to lock mutex")?;
    manager.list_sessions().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_volume(pid: u32, volume: f32, state: State<'_, AudioState>) -> Result<(), String> {
    let manager = state.0.lock().map_err(|_| "Failed to lock mutex")?;
    manager.set_volume(pid, volume).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_mute(pid: u32, muted: bool, state: State<'_, AudioState>) -> Result<(), String> {
    let manager = state.0.lock().map_err(|_| "Failed to lock mutex")?;
    manager.set_mute(pid, muted).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_system_volume(state: State<'_, AudioState>) -> Result<f32, String> {
    let manager = state.0.lock().map_err(|_| "Failed to lock mutex")?;
    manager.get_system_volume().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_system_volume(volume: f32, state: State<'_, AudioState>) -> Result<(), String> {
    let manager = state.0.lock().map_err(|_| "Failed to lock mutex")?;
    manager.set_system_volume(volume).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_system_mute(state: State<'_, AudioState>) -> Result<bool, String> {
    let manager = state.0.lock().map_err(|_| "Failed to lock mutex")?;
    manager.get_system_mute().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_system_mute(muted: bool, state: State<'_, AudioState>) -> Result<(), String> {
    let manager = state.0.lock().map_err(|_| "Failed to lock mutex")?;
    manager.set_system_mute(muted).map_err(|e| e.to_string())
}

fn main() {
    let audio_manager = AudioManager::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AudioState(Mutex::new(audio_manager)))
        .invoke_handler(tauri::generate_handler![
            greet,
            get_audio_sessions,
            set_volume,
            set_mute,
            get_system_volume,
            set_system_volume,
            get_system_mute,
            set_system_mute
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
