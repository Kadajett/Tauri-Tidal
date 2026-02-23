use objc2::rc::Retained;
use objc2::runtime::AnyObject;
use objc2_foundation::{NSMutableDictionary, NSNumber, NSString};
use objc2_media_player::{
    MPMediaItemPropertyAlbumTitle, MPMediaItemPropertyArtist, MPMediaItemPropertyPlaybackDuration,
    MPMediaItemPropertyTitle, MPNowPlayingInfoCenter, MPNowPlayingInfoPropertyElapsedPlaybackTime,
    MPNowPlayingInfoPropertyPlaybackRate, MPNowPlayingPlaybackState,
};

/// Update the macOS Now Playing info on the main thread.
/// macOS requires MPNowPlayingInfoCenter to be updated from the main thread
/// for the system to properly register the app as the Now Playing source.
pub fn update_now_playing(
    title: &str,
    artist: &str,
    album: &str,
    duration: f64,
    elapsed: f64,
    is_playing: bool,
) {
    let title = title.to_string();
    let artist = artist.to_string();
    let album = album.to_string();
    dispatch::Queue::main().exec_async(move || {
        set_now_playing_info(&title, &artist, &album, duration, elapsed, is_playing);
    });
}

/// Clear the Now Playing info on the main thread.
pub fn clear_now_playing() {
    dispatch::Queue::main().exec_async(move || {
        clear_now_playing_sync();
    });
}

/// Internal: set Now Playing info (must be called on main thread).
fn set_now_playing_info(
    title: &str,
    artist: &str,
    album: &str,
    duration: f64,
    elapsed: f64,
    is_playing: bool,
) {
    unsafe {
        let center = MPNowPlayingInfoCenter::defaultCenter();
        let dict: Retained<NSMutableDictionary<NSString, AnyObject>> = NSMutableDictionary::new();

        let title_val = NSString::from_str(title);
        let artist_val = NSString::from_str(artist);
        let album_val = NSString::from_str(album);
        let duration_val = NSNumber::new_f64(duration);
        let elapsed_val = NSNumber::new_f64(elapsed);
        let rate_val = NSNumber::new_f64(if is_playing { 1.0 } else { 0.0 });

        dict.insert(MPMediaItemPropertyTitle, &*title_val);
        dict.insert(MPMediaItemPropertyArtist, &*artist_val);
        dict.insert(MPMediaItemPropertyAlbumTitle, &*album_val);
        dict.insert(MPMediaItemPropertyPlaybackDuration, &*duration_val);
        dict.insert(MPNowPlayingInfoPropertyElapsedPlaybackTime, &*elapsed_val);
        dict.insert(MPNowPlayingInfoPropertyPlaybackRate, &*rate_val);

        center.setNowPlayingInfo(Some(&dict));
        center.setPlaybackState(if is_playing {
            MPNowPlayingPlaybackState::Playing
        } else {
            MPNowPlayingPlaybackState::Paused
        });
    }
}

/// Internal: clear Now Playing info (must be called on main thread).
fn clear_now_playing_sync() {
    unsafe {
        let center = MPNowPlayingInfoCenter::defaultCenter();
        center.setNowPlayingInfo(None);
        center.setPlaybackState(MPNowPlayingPlaybackState::Stopped);
    }
}
