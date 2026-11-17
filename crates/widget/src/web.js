// The browser build's storage and start form (docs/web.md), for
// src/web.rs. Everything is kept in one IndexedDB database:
//
// - "cd": the game's CD image the player picked, as the Blob the file
//   input gave (key "image": { blob, name, size }). IndexedDB keeps
//   Blobs without reading them into memory, in every browser and on the
//   page's own thread.
// - "files": the port's own files, the widgets the player saves
//   (*.WGT), WIDGET.INI and settings.ini, by their path below the
//   storage's mount point, as Uint8Arrays.
//
// Nothing is fetched or sent anywhere: the files come from the player.

const DB = "openwgt";
let opened = null;

function db() {
  if (!opened) {
    opened = new Promise((resolve, reject) => {
      const req = indexedDB.open(DB, 1);
      req.onupgradeneeded = () => {
        req.result.createObjectStore("cd");
        req.result.createObjectStore("files");
      };
      req.onsuccess = () => resolve(req.result);
      req.onerror = () => reject(req.error);
    });
  }
  return opened;
}

function request(req) {
  return new Promise((resolve, reject) => {
    req.onsuccess = () => resolve(req.result);
    req.onerror = () => reject(req.error);
  });
}

async function store(name, mode) {
  return (await db()).transaction(name, mode).objectStore(name);
}

function done(tx) {
  return new Promise((resolve, reject) => {
    tx.oncomplete = () => resolve();
    tx.onerror = () => reject(tx.error);
    tx.onabort = () => reject(tx.error);
  });
}

// The changes to "files", one after the other in the order the game
// made them.
let queue = Promise.resolve();

function change(f) {
  queue = queue
    .then(async () => {
      const s = await store("files", "readwrite");
      f(s);
      await done(s.transaction);
    })
    .catch((e) => console.error("storage:", e));
}

export function putFile(path, data) {
  // `data` is a view of the module's memory: copy it now.
  const copy = new Uint8Array(data);
  change((s) => s.put(copy, path));
}

export function deleteFile(path) {
  change((s) => s.delete(path));
}

export async function storedFiles() {
  await queue;
  const s = await store("files", "readonly");
  const [keys, values] = await Promise.all([request(s.getAllKeys()), request(s.getAll())]);
  return keys.map((k, i) => [String(k), new Uint8Array(values[i])]);
}

export async function readBlob(blob, start, end) {
  return new Uint8Array(await blob.slice(start, end).arrayBuffer());
}

export async function forgetCd() {
  const s = await store("cd", "readwrite");
  s.delete("image");
  await done(s.transaction);
}

function size(n) {
  return (n / (1024 * 1024)).toFixed(0) + " MB";
}

// The session's mark that the next start shows the form even with an
// image stored (Settings > Change CD...).
const CHANGE = "openwgt-change-cd";

// The start form's words in Swedish and French, for a browser in either.
// French puts a no-break space ( ) before ":".
const SV = {
  "t-cd": "Spelets skivavbildning (.iso)",
  "t-kept": "Sparad sedan förra gången:",
  "t-kept2": ". Välj en annan för att byta.",
  "t-note":
    "Widget Workshop: A Mad Scientist's Laboratory (Mac/PC-skivan), den svenska utgåvan " +
    "Uppfinnarverkstan eller den franska, Eurêka, le Laboratoire du savant fou. Avbildningen sparas " +
    "i den här webbläsaren och skickas ingenstans, precis som uppfinningarna du sparar. Uppfinningar " +
    "som dras till spelet öppnas, och Arkiv ▸ Spara för nedladdning och Ladda ner en uppfinning ger " +
    "dig dem igen. Arkiv ▸ Kopiera länk till uppfinningen gör en länk som öppnar uppfinningen för " +
    "alla som har skivan.",
  open: "Spela utan skivan",
  start: "Starta",
  "t-open-note": "Utan skivan använder spelet OpenWGT:s egna bilder, ljud, texter och pussel, på",
  "t-cd-lang": "Med skivan talar det skivans språk.",
};
const FR = {
  "t-cd": "L'image du CD du jeu (.iso)",
  "t-kept": "Gardée depuis la dernière fois :",
  "t-kept2": ". Choisis-en une autre pour la remplacer.",
  "t-note":
    "Widget Workshop: A Mad Scientist's Laboratory (le CD Mac/PC), sa version suédoise, " +
    "Uppfinnarverkstan, ou sa version française, Eurêka, le Laboratoire du savant fou. L'image " +
    "reste dans ce navigateur et n'est envoyée nulle part, tout comme les Eurêkas que tu " +
    "enregistres. Les Eurêkas glissés sur le jeu s'ouvrent, et Fichier ▸ Enregistrer pour " +
    "téléchargement et Télécharger un Eurêka te les rendent. Fichier ▸ Copier le lien vers " +
    "l'Eurêka crée un lien qui ouvre l'Eurêka pour tous ceux qui ont le CD.",
  open: "Jouer sans le CD",
  start: "Démarrer",
  "t-open-note": "Sans le CD, le jeu utilise les images, sons, textes et problèmes d'OpenWGT, en",
  "t-cd-lang": "Avec le CD, il parle la langue du CD.",
};
const WORDS = { sv: SV, fr: FR };

// The page's own words outside the form, in every language.
const TURN = {
  en: "Turn the phone sideways for a bigger workshop.",
  sv: "Vänd telefonen på sidan för en större verkstad.",
  fr: "Tourne le téléphone sur le côté pour un atelier plus grand.",
};
const LANGUAGE = { en: "Language", sv: "Språk", fr: "Langue" };

// Puts the page in the language `code`: its `lang` (which the touch
// screen's text box follows, platform::web) and the words outside the
// form. The form's language first, the game's once it starts.
export function pageLanguage(code) {
  if (!TURN[code]) return;
  document.documentElement.lang = code;
  const turn = document.getElementById("turn");
  if (turn) turn.textContent = TURN[code];
  const lang = document.getElementById("lang");
  if (lang) lang.setAttribute("aria-label", LANGUAGE[code]);
}

// The form in the browser's language, and its language list, which
// "Play without the CD" follows (`?open&lang=`): the one picked last
// time, else the browser's.
function formLanguage($) {
  const browser = (navigator.language || "").toLowerCase().slice(0, 2);
  const words = WORDS[browser];
  if (words) {
    for (const [id, text] of Object.entries(words)) {
      const el = $(id);
      if (el) el.textContent = text;
    }
  }
  pageLanguage(words ? browser : "en");
  const lang = $("lang");
  const open = $("open");
  if (!lang || !open) return;
  let picked = null;
  try {
    picked = localStorage.getItem("openwgt-lang");
  } catch (e) {
    console.error("storage:", e);
  }
  lang.value = picked || (words ? browser : "en");
  const follow = () => {
    open.href = `?open&lang=${lang.value}`;
    try {
      localStorage.setItem("openwgt-lang", lang.value);
    } catch (e) {
      console.error("storage:", e);
    }
  };
  lang.onchange = follow;
  follow();
}

// Reloads the page to play without the CD in the language `code`
// (Settings ▸ Language), kept for the start form's list too.
export function changeLang(code) {
  try {
    localStorage.setItem("openwgt-lang", code);
  } catch (e) {
    console.error("storage:", e);
  }
  location.search = `?open&lang=${code}`;
}

// Reloads the page into the start form, the stored image kept: Start
// without picking another goes on with it.
export function changeCd() {
  try {
    sessionStorage.setItem(CHANGE, "1");
  } catch (e) {
    console.error("storage:", e);
  }
  location.reload();
}

// Resolves to the CD image: the stored one at once, else (or after
// changeCd) once the player presses Start in the start form (#setup)
// with one. A newly picked image is stored first. `error` says why the
// last one didn't do. The game's sound starts with the first click or
// key either way (platform::audio::resume).
export async function chooseCd(error) {
  const $ = (id) => document.getElementById(id);
  formLanguage($);
  let stored = null;
  try {
    stored = await request((await store("cd", "readonly")).get("image"));
  } catch (e) {
    console.error("storage:", e);
  }
  let change = false;
  try {
    change = sessionStorage.getItem(CHANGE) !== null;
    sessionStorage.removeItem(CHANGE);
  } catch (e) {
    console.error("storage:", e);
  }
  if (stored && !change && !error) return stored.blob;
  $("setup").hidden = false;
  $("setup-error").textContent = error || "";
  $("setup-error").hidden = !error;
  $("cd-stored").hidden = !stored;
  if (stored) $("cd-name").textContent = `${stored.name} (${size(stored.size)})`;
  const cd = $("cd-file");
  cd.value = "";
  const update = () => ($("start").disabled = !(stored || cd.files.length));
  cd.onchange = update;
  update();
  $("start").focus();
  return new Promise((resolve) => {
    $("start").onclick = async () => {
      $("start").disabled = true;
      let blob = stored && stored.blob;
      const file = cd.files[0];
      const note = [];
      if (file) {
        blob = file;
        try {
          const s = await store("cd", "readwrite");
          s.put({ blob: file, name: file.name, size: file.size }, "image");
          await done(s.transaction);
        } catch (e) {
          console.error("storage:", e);
          note.push("The browser didn't keep the CD image; it asks again next time.");
        }
      }
      // Asks the browser not to clear the storage when space runs low.
      if (navigator.storage && navigator.storage.persist) navigator.storage.persist().catch(() => {});
      for (const n of note) console.warn(n);
      $("setup").hidden = true;
      resolve(blob);
    };
  });
}
