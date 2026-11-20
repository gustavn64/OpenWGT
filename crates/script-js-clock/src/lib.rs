//! QuickJS's clock in the browser build.
//!
//! On `wasm32-unknown-unknown` QuickJS (rquickjs-sys's shim) reads the
//! time through one import, `env.__rquickjs_host_now_us`, which the page
//! can't give: wasm-bindgen's glue would import it from a module `env`
//! that doesn't exist. Defining the function in the module itself
//! resolves it at link time. Scripts never see this clock (`Date` is the
//! game's), so it stands still.
//!
//! The definition needs `#[unsafe(no_mangle)]`, hence a crate of its own
//! with its own lints, the rest of the workspace keeping
//! `unsafe_code = "forbid"`.

/// Keeps this crate in the link; `script-js` calls it once.
pub fn link() {}

/// Microseconds since the Unix epoch, as QuickJS's shim asks: always 0.
#[cfg(target_arch = "wasm32")]
#[allow(unsafe_code)] // `no_mangle`: the name QuickJS's shim imports
#[unsafe(no_mangle)]
pub extern "C" fn __rquickjs_host_now_us() -> f64 {
    0.0
}
