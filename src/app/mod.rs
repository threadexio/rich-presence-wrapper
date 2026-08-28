mod common;

#[cfg(feature = "helix")]
pub mod helix;

#[cfg(feature = "zed")]
pub mod zed;

#[cfg(feature = "mpris-bridge")]
pub mod mpris_bridge;

#[cfg(feature = "music-bridge")]
pub mod music_bridge;

#[cfg(feature = "lsp")]
pub mod lsp;
