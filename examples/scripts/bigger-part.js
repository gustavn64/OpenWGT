// A scripted part of your own: it gives the bigger of its two inputs.
// With the debugging tools on, drop this file onto the window (a selected
// scripted part takes it, else a new one does), or paste it into a
// scripted part's editor (a double-click on the part) and press Apply.

definePart({
  name: "Bigger",
  inputs: ["a", "b"],
  outputs: ["out"],
  tick(state, { a, b }) {
    // An input with nothing on it is null.
    if (a === null) return { out: b };
    if (b === null) return { out: a };
    return { out: a > b ? a : b };
  },
});
