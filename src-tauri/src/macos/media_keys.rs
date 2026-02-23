use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_media_player::{
    MPRemoteCommandCenter, MPRemoteCommandEvent, MPRemoteCommandHandlerStatus,
};
use std::ptr::NonNull;

/// Event names emitted by media key handlers
pub const MEDIA_KEY_TOGGLE_PLAY: &str = "media-key:toggle-play";
pub const MEDIA_KEY_NEXT: &str = "media-key:next";
pub const MEDIA_KEY_PREVIOUS: &str = "media-key:previous";

/// Register media key handlers with the system.
/// Returns tokens that MUST be kept alive for the handlers to remain active.
pub fn register_media_key_handlers(app_handle: tauri::AppHandle) -> Vec<Retained<AnyObject>> {
    let mut tokens = Vec::new();

    unsafe {
        let command_center = MPRemoteCommandCenter::sharedCommandCenter();

        // Play command
        let play_cmd = command_center.playCommand();
        play_cmd.setEnabled(true);
        let handle = app_handle.clone();
        let play_block = RcBlock::new(
            move |_event: NonNull<MPRemoteCommandEvent>| -> MPRemoteCommandHandlerStatus {
                use tauri::Emitter;
                let _ = handle.emit(MEDIA_KEY_TOGGLE_PLAY, "play");
                MPRemoteCommandHandlerStatus::Success
            },
        );
        tokens.push(play_cmd.addTargetWithHandler(&play_block));

        // Pause command
        let pause_cmd = command_center.pauseCommand();
        pause_cmd.setEnabled(true);
        let handle = app_handle.clone();
        let pause_block = RcBlock::new(
            move |_event: NonNull<MPRemoteCommandEvent>| -> MPRemoteCommandHandlerStatus {
                use tauri::Emitter;
                let _ = handle.emit(MEDIA_KEY_TOGGLE_PLAY, "pause");
                MPRemoteCommandHandlerStatus::Success
            },
        );
        tokens.push(pause_cmd.addTargetWithHandler(&pause_block));

        // Toggle play/pause command
        let toggle_cmd = command_center.togglePlayPauseCommand();
        toggle_cmd.setEnabled(true);
        let handle = app_handle.clone();
        let toggle_block = RcBlock::new(
            move |_event: NonNull<MPRemoteCommandEvent>| -> MPRemoteCommandHandlerStatus {
                use tauri::Emitter;
                let _ = handle.emit(MEDIA_KEY_TOGGLE_PLAY, "toggle");
                MPRemoteCommandHandlerStatus::Success
            },
        );
        tokens.push(toggle_cmd.addTargetWithHandler(&toggle_block));

        // Next track command
        let next_cmd = command_center.nextTrackCommand();
        next_cmd.setEnabled(true);
        let handle = app_handle.clone();
        let next_block = RcBlock::new(
            move |_event: NonNull<MPRemoteCommandEvent>| -> MPRemoteCommandHandlerStatus {
                use tauri::Emitter;
                let _ = handle.emit(MEDIA_KEY_NEXT, ());
                MPRemoteCommandHandlerStatus::Success
            },
        );
        tokens.push(next_cmd.addTargetWithHandler(&next_block));

        // Previous track command
        let prev_cmd = command_center.previousTrackCommand();
        prev_cmd.setEnabled(true);
        let handle = app_handle.clone();
        let prev_block = RcBlock::new(
            move |_event: NonNull<MPRemoteCommandEvent>| -> MPRemoteCommandHandlerStatus {
                use tauri::Emitter;
                let _ = handle.emit(MEDIA_KEY_PREVIOUS, ());
                MPRemoteCommandHandlerStatus::Success
            },
        );
        tokens.push(prev_cmd.addTargetWithHandler(&prev_block));
    }

    log::info!("Media key handlers registered ({} tokens)", tokens.len());
    tokens
}
