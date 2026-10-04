//! The panic boundary around every call into a surface (B9; shell.md rule 2).

use std::{
    cell::Cell,
    panic::{AssertUnwindSafe, catch_unwind},
};

thread_local! {
    static SURFACE: Cell<Option<&'static str>> = const { Cell::new(None) };
}

/// The surface whose code is running on this thread, if any. The compositor's panic hook reads
/// it to log a surface's panic instead of recording a crash of the core.
pub fn panicking_surface() -> Option<&'static str> {
    SURFACE.get()
}

/// Runs `f` as surface `name`'s code; a panic in it comes back as its message.
pub fn guard<T>(name: &'static str, f: impl FnOnce() -> T) -> Result<T, String> {
    let outer = SURFACE.replace(Some(name));
    let result = catch_unwind(AssertUnwindSafe(f));
    SURFACE.set(outer);
    result.map_err(|payload| {
        payload
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "(no message)".into())
    })
}
