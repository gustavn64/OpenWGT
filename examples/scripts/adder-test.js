// The half adder of adder-test.txt in JavaScript, as tests: each pair of
// inputs a test of its own, a ✓ or a ✗ for each.
//
//   openwgt --open --headless --script examples/scripts/adder-test.js

/// <reference path="../../crates/script-js/openwgt-script.d.ts" />
// @ts-check

open("ADDER.WGT");
const a = part("A");
const b = part("B");
const sum = part("#11");
const carry = part("#10");
run();

for (const [x, y] of [[0, 0], [1, 0], [0, 1], [1, 1]]) {
  test(`${x} + ${y}`, async () => {
    // Setting a switch's value works the switch.
    a.value = x === 1;
    b.value = y === 1;
    await ticks(2);
    assertEqual(sum.on, (x ^ y) === 1, "the sum");
    assertEqual(carry.on, (x & y) === 1, "the carry");
  });
}
