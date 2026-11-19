//! The browser's side of the platform (`wasm32`): the console for what
//! the port would print on the terminal, the page's status line for
//! what the player has to see, files dropped onto the page, files
//! handed to the player as downloads, and the clipboard
//! ([`crate::clipboard`]).

use std::cell::RefCell;

use wasm_bindgen::JsCast;
use wasm_bindgen::prelude::Closure;

use crate::DroppedFile;

/// Writes `text` to the browser's console, where `eprintln!` goes in the
/// browser build.
pub fn log(text: &str) {
    web_sys::console::log_1(&text.into());
}

/// Writes `text` to the console as an error and shows it on the page, in
/// the element with the id `status` if there is one.
pub fn show_error(text: &str) {
    web_sys::console::error_1(&text.into());
    show_status(text, true);
}

/// Shows `text` on the page's status line (`#status`), as an error or
/// not; an empty text hides it.
pub fn show_status(text: &str, error: bool) {
    let Some(el) = element("status") else { return };
    el.set_text_content(Some(text));
    let _ = el.set_attribute("class", if error { "error" } else { "" });
    let _ = el.set_attribute("style", if text.is_empty() { "display:none" } else { "" });
}

/// The page's element with the id `id`.
#[must_use]
pub fn element(id: &str) -> Option<web_sys::Element> {
    web_sys::window()?.document()?.get_element_by_id(id)
}

/// `navigator.clipboard`, if the page may have it (only over HTTPS or
/// from `localhost`).
pub(crate) fn clipboard() -> Option<web_sys::Clipboard> {
    let navigator = web_sys::window()?.navigator();
    let c = js_sys::Reflect::get(&navigator, &"clipboard".into()).ok()?;
    (!c.is_undefined()).then(|| c.unchecked_into())
}

/// The page's `<canvas id="widget">`, if it has one.
pub(crate) fn canvas() -> Option<web_sys::HtmlCanvasElement> {
    element("widget")?.dyn_into().ok()
}

thread_local! {
    /// The files dropped and read, for [`take_drops`]; `None` until
    /// [`watch_drops`].
    static DROPS: RefCell<Option<Vec<DroppedFile>>> = const { RefCell::new(None) };
}

/// Takes the files dropped onto the page from then on (once): the
/// browser would open one in place of the page. Dropped files are read
/// and kept for [`take_drops`].
pub(crate) fn watch_drops() {
    if DROPS.with_borrow(Option::is_some) {
        return;
    }
    DROPS.set(Some(Vec::new()));
    let Some(window) = web_sys::window() else { return };
    let over = Closure::<dyn FnMut(web_sys::DragEvent)>::new(|e: web_sys::DragEvent| {
        e.prevent_default();
        if let Some(dt) = e.data_transfer() {
            dt.set_drop_effect("copy");
        }
    });
    let on_drop = Closure::<dyn FnMut(web_sys::DragEvent)>::new(|e: web_sys::DragEvent| {
        e.prevent_default();
        let Some(files) = e.data_transfer().and_then(|dt| dt.files()) else {
            return;
        };
        for i in 0..files.length() {
            let Some(file) = files.get(i) else { continue };
            wasm_bindgen_futures::spawn_local(async move {
                match wasm_bindgen_futures::JsFuture::from(file.array_buffer()).await {
                    Ok(buf) => {
                        let data = js_sys::Uint8Array::new(&buf).to_vec();
                        let f = DroppedFile {
                            name: file.name(),
                            path: None,
                            data,
                        };
                        DROPS.with_borrow_mut(|d| d.get_or_insert_default().push(f));
                    }
                    Err(e) => web_sys::console::error_2(&"reading the dropped file:".into(), &e),
                }
            });
        }
    });
    let _ = window.add_event_listener_with_callback("dragover", over.as_ref().unchecked_ref());
    let _ = window.add_event_listener_with_callback("drop", on_drop.as_ref().unchecked_ref());
    // The page's for good.
    over.forget();
    on_drop.forget();
}

/// The files dropped and read since last time.
pub(crate) fn take_drops() -> Vec<DroppedFile> {
    DROPS.with_borrow_mut(|d| d.as_mut().map_or_default(std::mem::take))
}

/// Hands `data` to the player as a download named `name` (an `<a
/// download>` on a `Blob` of it); false if the browser wouldn't.
#[must_use]
pub fn download(name: &str, data: &[u8]) -> bool {
    let go = || -> Option<()> {
        let parts = js_sys::Array::of1(&js_sys::Uint8Array::from(data));
        let opts = web_sys::BlobPropertyBag::new();
        opts.set_type("application/octet-stream");
        let blob = web_sys::Blob::new_with_u8_array_sequence_and_options(&parts, &opts).ok()?;
        let url = web_sys::Url::create_object_url_with_blob(&blob).ok()?;
        let document = web_sys::window()?.document()?;
        let a: web_sys::HtmlAnchorElement = document.create_element("a").ok()?.dyn_into().ok()?;
        a.set_href(&url);
        a.set_download(name);
        let body = document.body()?;
        let _ = body.append_child(&a);
        a.click();
        a.remove();
        // After the browser has started the download.
        let revoke = Closure::once_into_js(move || {
            let _ = web_sys::Url::revoke_object_url(&url);
        });
        let _ =
            web_sys::window()?.set_timeout_with_callback_and_timeout_and_arguments_0(revoke.unchecked_ref(), 60_000);
        Some(())
    };
    go().is_some()
}

// A touch screen's keyboard: a hidden text field that has the focus while
// the game wants key presses (`App::wants_text`: a running Keypress),
// focused from a touch (the
// browsers bring up their keyboard only for one); what's typed into it
// becomes the game's key presses, as Windows' virtual-key codes with
// Shift around them (`editor::prompt::typed`'s keys), the key let go a
// little later so that a Keypress part sees it held.
#[wasm_bindgen::prelude::wasm_bindgen(inline_js = r#"
const KEEP = "~~";
const SPECIAL = { Backspace: 8, Tab: 9, Enter: 13, Escape: 27, End: 35, Home: 36,
  ArrowLeft: 37, ArrowUp: 38, ArrowRight: 39, ArrowDown: 40, Delete: 46 };
const SHIFTED_DIGITS = ")!@#$%^&*(";
const PUNCT = { " ": [32, false], ".": [0xbe, false], ",": [0xbc, false], "-": [0xbd, false],
  "_": [0xbd, true], "/": [0xbf, false], "\\": [0xdc, false], ":": [0xba, true], ";": [0xba, false],
  "'": [0xde, false], "=": [0xbb, false], "+": [0xbb, true], "[": [0xdb, false], "]": [0xdd, false],
  "~": [0xc0, true], "\n": [13, false] };
let field = null, canvas = null, wanted = false, touched = -1e9, keys = [];

function key(vk, shift) {
  if (shift) keys.push(16, 1);
  keys.push(vk, 1);
  if (shift) keys.push(16, 0);
  setTimeout(() => keys.push(vk, 0), 80);
}
function typed(c) {
  const code = c.charCodeAt(0);
  if (c >= "a" && c <= "z") return key(code - 32, false);
  if (c >= "A" && c <= "Z") return key(code, true);
  if (c >= "0" && c <= "9") return key(code, false);
  const d = SHIFTED_DIGITS.indexOf(c);
  if (d >= 0) return key(48 + d, true);
  const p = PUNCT[c];
  if (p) key(p[0], p[1]);
}
function reset() {
  field.value = KEEP;
  try { field.setSelectionRange(KEEP.length, KEEP.length); } catch (_) {}
}
function changed() {
  const v = field.value;
  let same = 0;
  while (same < KEEP.length && same < v.length && v[same] === KEEP[same]) same++;
  for (let i = same; i < KEEP.length; i++) key(8, false);
  for (const c of v.slice(same)) typed(c);
  reset();
}
function focus() {
  field.focus({ preventScroll: true });
  reset();
}
export function text_setup(c) {
  if (field) return;
  canvas = c;
  field = document.createElement("input");
  field.type = "text";
  for (const [k, v] of [["autocomplete", "off"], ["autocapitalize", "off"], ["autocorrect", "off"],
      ["spellcheck", "false"], ["aria-hidden", "true"], ["tabindex", "-1"]]) field.setAttribute(k, v);
  field.style.cssText = "position:fixed;left:0;bottom:0;width:1px;height:1px;opacity:0;border:0;" +
    "padding:0;margin:0;font-size:16px;pointer-events:none";
  document.body.appendChild(field);
  reset();
  field.addEventListener("input", (e) => { if (!e.isComposing) changed(); });
  field.addEventListener("compositionend", changed);
  field.addEventListener("keydown", (e) => {
    const vk = SPECIAL[e.key];
    if (vk !== undefined) { e.preventDefault(); key(vk, false); return; }
    // A keyboard's shortcuts (Ctrl+S) still reach the game.
    if ((e.ctrlKey || e.metaKey) && e.key.length === 1) {
      const c = e.key.toUpperCase().charCodeAt(0);
      if ((c >= 65 && c <= 90) || (c >= 48 && c <= 57)) {
        e.preventDefault();
        keys.push(17, 1, c, 1, c, 0, 17, 0);
      }
    }
  });
  canvas.addEventListener("pointerup", (e) => {
    if (e.pointerType === "mouse") return;
    touched = performance.now();
    if (wanted) focus();
  });
}
export function text_want(on) {
  if (!field || on === wanted) return;
  wanted = on;
  if (on) {
    if (performance.now() - touched < 1500) focus();
  } else if (document.activeElement === field) {
    field.blur();
    canvas.focus({ preventScroll: true });
  }
}
let edited = [];
// A text field tapped on a touch screen: the browser's own text box over
// the game, with the field's text, so the device's keyboard (its letters,
// corrections and selection) edits it; OK hands it back. Cancel is in
// the page's language (its `lang`, the game's: the widget crate's web.js).
export function text_edit(text, multiline, max) {
  const back = document.createElement("div");
  back.style.cssText = "position:fixed;inset:0;z-index:10;display:flex;align-items:flex-start;" +
    "justify-content:center;padding-top:8vh;background:rgba(0,0,0,.35)";
  const form = document.createElement("form");
  form.style.cssText = "box-sizing:border-box;width:min(92vw,32em);padding:12px;background:#c0c0c0;" +
    "border:2px solid;border-color:#fff #000 #000 #fff;font:16px system-ui,sans-serif";
  const field = document.createElement(multiline ? "textarea" : "input");
  if (multiline) field.rows = 5;
  field.value = text;
  if (max > 0) field.maxLength = max;
  field.style.cssText = "box-sizing:border-box;width:100%;padding:6px;font:16px system-ui,sans-serif";
  const row = document.createElement("div");
  row.style.cssText = "display:flex;justify-content:flex-end;gap:8px;margin-top:10px";
  const button = (label, type) => {
    const b = document.createElement("button");
    b.type = type;
    b.textContent = label;
    b.style.cssText = "min-width:6em;padding:.4em 1em;font:16px system-ui,sans-serif";
    return b;
  };
  const cancelWord = { sv: "Avbryt", fr: "Annuler" }[document.documentElement.lang] || "Cancel";
  const cancel = button(cancelWord, "button"), ok = button("OK", "submit");
  row.append(cancel, ok);
  form.append(field, row);
  back.append(form);
  document.body.append(back);
  const close = (keep) => {
    if (keep) edited.push(field.value);
    back.remove();
    if (canvas) canvas.focus({ preventScroll: true });
  };
  form.addEventListener("submit", (e) => { e.preventDefault(); close(true); });
  form.addEventListener("keydown", (e) => { if (e.key === "Escape") close(false); });
  cancel.addEventListener("click", () => close(false));
  back.addEventListener("pointerdown", (e) => { if (e.target === back) close(false); });
  field.focus();
  if (!multiline) field.select();
}
export function text_edited() {
  const e = edited;
  edited = [];
  return e;
}
export function coarse_pointer() {
  return window.matchMedia("(pointer: coarse)").matches;
}
export function text_take() {
  const k = keys;
  keys = [];
  return new Int32Array(k);
}
"#)]
extern "C" {
    fn text_setup(canvas: &web_sys::HtmlCanvasElement);
    fn text_want(on: bool);
    fn text_take() -> js_sys::Int32Array;
    fn text_edit(text: &str, multiline: bool, max: u32);
    fn text_edited() -> js_sys::Array;
    fn coarse_pointer() -> bool;
}

/// Edits a text field's text in the browser's own text box (a touch
/// screen's): [`take_edited`] has it once OK is pressed.
pub(crate) fn edit_text(field: &crate::TextEdit) {
    text_edit(&field.text, field.multiline, u32::try_from(field.max).unwrap_or(0));
}

/// The texts edited in the browser's text box since last time.
pub(crate) fn take_edited() -> Vec<String> {
    text_edited().iter().filter_map(|v| v.as_string()).collect()
}

/// Whether the main pointer is a finger (a phone's or a tablet's).
pub(crate) fn touch_screen() -> bool {
    coarse_pointer()
}

/// Sets up the touch screen's keyboard field for `canvas` (once).
pub(crate) fn setup_text(canvas: &web_sys::HtmlCanvasElement) {
    text_setup(canvas);
}

/// Whether the game wants typing now: the keyboard comes up (after a
/// touch) or goes.
pub(crate) fn want_text(on: bool) {
    text_want(on);
}

/// The keys typed on the touch screen's keyboard since last time: (code,
/// pressed).
pub(crate) fn take_typed() -> Vec<(u16, bool)> {
    text_take()
        .to_vec()
        .as_chunks::<2>()
        .0
        .iter()
        .filter_map(|&[vk, down]| Some((u16::try_from(vk).ok()?, down != 0)))
        .collect()
}
