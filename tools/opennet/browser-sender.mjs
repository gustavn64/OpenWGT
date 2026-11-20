// A Sender in the browser, end to end (docs/net-parts.md): listens on the
// relay as a Receiver, opens the game headless in Chromium with a widget
// (a number wired to a Sender, from the engine's ignored test
// browser_sender_link), clicks the worksheet, presses Enter (GO) and
// waits for the number.
//   node browser-sender.mjs <relay wss://...> <channel> <page url> <chromium> <number>
// TMPDIR must be short: Chromium's socket path has to fit in 108 bytes.
import { spawn } from "node:child_process";

const [relay, channel, url, chromium, want] = process.argv.slice(2);
const log = (...a) => console.log(new Date().toISOString().slice(11, 19), ...a);
const fail = (m) => { log("FAIL:", m); process.exit(1); };
setTimeout(() => fail("timed out"), 90_000);

const seen = [];
const rx = new WebSocket(`${relay}/${channel}`);
rx.onopen = () => rx.send(JSON.stringify({ t: "hello", role: "recv", run: true, v: 1 }));
rx.onerror = (e) => fail("relay: " + e.message);
rx.onmessage = (m) => {
  log("relay ->", m.data);
  const msg = JSON.parse(m.data);
  seen.push(msg);
  if (msg.t === "val" && String(msg.v) === want) {
    log("OK: the browser's Sender sent", msg.v, "; peer states seen:",
      seen.filter((x) => x.t === "peer").map((x) => x.s).join(" "));
    browser.kill();
    process.exit(0);
  }
};

const browser = spawn(chromium, ["--headless=new", "--no-sandbox", "--remote-debugging-port=9333",
  "--enable-unsafe-swiftshader", "--window-size=1024,768",
  "--user-data-dir=" + (process.env.TMPDIR || "/tmp") + "/chromium-sender", url], { stdio: ["ignore", "ignore", "pipe"] });
browser.stderr.on("data", (d) => { if (/error/i.test(d) && !/dbus|gpu|Fontconfig/i.test(d)) log("chromium:", String(d).trim().slice(0, 200)); });

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
let page;
for (let i = 0; i < 50 && !page; i++) {
  await sleep(300);
  try {
    const list = await (await fetch("http://127.0.0.1:9333/json")).json();
    page = list.find((p) => p.type === "page");
  } catch {}
}
if (!page) fail("no page");
const cdp = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((r) => (cdp.onopen = r));
let id = 0;
const pending = new Map();
cdp.onmessage = (m) => {
  const msg = JSON.parse(m.data);
  if (msg.id && pending.has(msg.id)) { pending.get(msg.id)(msg); pending.delete(msg.id); }
  if (msg.method === "Runtime.consoleAPICalled") log("console:", msg.params.args.map((a) => a.value).join(" ").slice(0, 200));
  if (msg.method === "Runtime.exceptionThrown") log("exception:", JSON.stringify(msg.params.exceptionDetails).slice(0, 300));
};
const send = (method, params = {}) => new Promise((r) => { id++; pending.set(id, r); cdp.send(JSON.stringify({ id, method, params })); });
await send("Runtime.enable");
log("page open; waiting for the game to load");
await sleep(15_000);
// A click on the worksheet (empty there) gives the canvas the keyboard; Enter is GO.
await send("Input.dispatchMouseEvent", { type: "mousePressed", x: 800, y: 500, button: "left", clickCount: 1 });
await send("Input.dispatchMouseEvent", { type: "mouseReleased", x: 800, y: 500, button: "left", clickCount: 1 });
await sleep(500);
for (const type of ["keyDown", "keyUp"]) {
  await send("Input.dispatchKeyEvent", { type, key: "Enter", code: "Enter", windowsVirtualKeyCode: 13, nativeVirtualKeyCode: 13, ...(type === "keyDown" ? { text: "\r" } : {}) });
}
log("pressed Enter (GO)");
