use syre_core as core;
use syre_local::error::IoSerde;

/// # Returns
/// User count if user manifest is `Ok`,
/// otherwise `Err`.
pub async fn count() -> Result<usize, ()> {
    tauri_sys::core::invoke_result("user_count", ()).await
}

/// Get the active user.
pub async fn fetch_user() -> Result<Option<core::system::User>, IoSerde> {
    tauri_sys::core::invoke_result("active_user", ()).await
}
