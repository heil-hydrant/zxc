/// Returns whether a frame should be updated.
///
/// The user sets `b:update` in the interceptor and repeater windows.
/// HTTP only. WebSocket frames are always updated, so the blanket
/// implementation for WebSocket always returns `true`.
pub trait ShouldUpdate {
    fn should_update(&self) -> bool;
}
