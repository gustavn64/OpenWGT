# OpenWSFX: the port's own sounds

OpenWSFX is a set of new sounds for Widget Workshop, made for the port,
that stands in for the CD's (`WWSOUND.DLL`, type 1006) when the port is
started with `--opensfx` (`engine::opensfx`), as OpenSFX does for
Transport Tycoon Deluxe. Each sound keeps its number and its job: the
Animal Sound's dog barks, the switch clicks, the Speaker's tone samples
are 220 Hz and 1760 Hz. The sounds themselves are synthesized anew. The
game has no music outside its intro, which the port leaves out, so there
is no OpenMSX counterpart.

The pack lives in `crates/engine/opensfx/sounds/<id>.wav` and is built
into the program; `--opensfx=DIR` reads a folder of the same layout. A
sound the pack leaves out is the CD's.

## Making it

```sh
nix-shell -p 'python3.withPackages(p: [p.numpy p.scipy p.pillow])'
tools/opensfx/make.py                      # every sound
tools/opensfx/make.py 200 201 --preview /tmp/sv --compare EXPORT
```

Every sound is a Python function (`sounds/*.py`) built with the kit in
`sfx.py`: oscillators, noise, filters, envelopes, formants, FM. `--compare`
draws the CD's sound (from `openwgt --export=EXPORT CD.iso`) under the new
one's waveform and spectrogram, to match length, loudness and pitch.

## Rules

- **Make, don't sample.** Nothing of the CD's sounds goes into a new one:
  no samples, no resampling, no filtering of them. Look at them (length,
  envelope, pitch) to learn what the sound is; then make it from scratch.
- **Keep the job.** The same length within reason (the parts' timing
  doesn't wait for a sound, but a sound much longer than its event
  overlaps the next), the same kind of event, recognisable at once.
- **Keep the tone samples exact.** 1006/220 and 1006/224 are looped and
  resampled for every pitch the Speaker plays: a whole number of cycles
  (2205 samples is 44 cycles of 220 Hz), steady, no fade.
- 8-bit mono 11025 Hz, as the game's: keep the sounds clean and not too
  bright; there's little room above 4 kHz.

## Sounds

| ids | what |
|---|---|
| 128–150 | the editor: click, wire, select, pick, drop, delete, dialogs, Light, GO, STOP, Sound on/off, placing, arrows, sliding |
| 200–203 | Animal Sound: dog, cat, rooster, cow |
| 205, 220, 224 | the default tone; the Speaker's tone samples |
| 300–309 | Misc Sound: buzzer, alarm, chime, clap, breath, laugh, cuckoo, weird, spooky, chord |
| 1000–1151 | the parts' own: pendulum, gravity chamber, cannon, coin, random number, tuning fork, metronome, switch, heart, crunchers, keypad, switcher, plunger, win/lose |
