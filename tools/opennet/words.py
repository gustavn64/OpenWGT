#!/usr/bin/env python3
"""Picks the word list the network parts' random channel names are made of
(src/words.txt): 2048 short common English words, three of which, joined
with hyphens, make a name like `cat-sun-oak` (2048³, about 8.6 billion).

    words.py SCOWL_DICT_DIR > src/words.txt     # pick the list again
    words.py --check src/words.txt [SCOWL_DICT_DIR]   # check a list by hand

The words are to be read out over the phone and typed by children, so:

- 3 to 5 letters, a to z only, and in American English's common words
  (SCOWL's size 35 list, `nixpkgs#scowl`), with a vowel;
- common: the most frequent first (`wordfreq`);
- one form of a word (no plurals, past tenses, -ing or -ly forms of
  another), no little words (the, and, with...), no number or letter
  names;
- said only one way and spelt only one way: none that sounds like another
  common word (sun/son, sea/see, grey/gray), by the CMU Pronouncing
  Dictionary (`python3Packages.cmudict`), and none with silent or
  unusual letters (knot, lamb, light, phone);
- none that sounds like the start of another in the list (car/card), or
  like another but for one easily confused sound (cab/cap, bad/bat,
  sum/sun), so that a name heard over a bad line is still one name;
- nothing rude, scary or grown-up (the BLOCK list, and a read-through).

Needs `nix-shell -p 'python3.withPackages(p: [p.wordfreq p.cmudict])' scowl`.
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

COUNT = 2048
# The SCOWL size the words come from.
POOL = 50

# Little words: fine words, poor names.
STOP = set("""
the and but for nor yet with from into onto upon over under than then that this these those there their
they them what when where which while who whom whose why how you your yours our ours his her hers its
was were are has had have having been being does did done doing can could would should shall will may might
must not any all each few many much more most some such own same other only very just also too here
about above after again against below down off out once both either neither else ever every whether
unto till until via per off ago etc yes yeah okay hey hello bye nope well lot lots thing stuff
she him one two three four five six seven eight nine ten zero none plus minus
mr mrs ms dr st oh ah uh um hmm wow ouch ok
""".split())

# Rude, scary, grown-up or touchy, or a name.
BLOCK = set(("""
ass arse butt bum boob bra tit tits dick cock cum fag gay homo sex sexy nude porn rape slut whore pimp
crap shit piss poo poop pee wee fart puke vomit snot spit barf turd
damn hell fuck suck sucks lick hump horny kinky
kill dead die died dies death dying gun guns bomb bombs knife stab shoot shot slay war wars blood bloody
gore corpse grave tomb coffin skull bones fight fist punch kick slap hit hurt harm pain wound bleed
drug drugs dope weed pot beer wine vodka rum gin booze drunk smoke cigar cocaine meth
nazi jew jews arab islam hindu bible pray pope god gods devil demon satan sin sinful jesus christ church
ugly fat fatty dumb stupid idiot moron loser jerk nerd dork lame freak creep weird crazy mad psycho
hate hated sick ill ache cancer virus germ flu lice
racist slave whip chain cage jail prison cop cops thief steal rob crook
money cash rich poor debt tax taxes bank loan
bitch witch hag
naked strip bed kiss lust date flirt
cunt pussy penis sperm pubic anal anus orgy porno lewd harem thong booby bosom potty bong spank queer sissy
butch swine trans woke trump nuke ammo rifle taser arson felon fraud bribe crime theft thug gang lynch slain
dirk shank goon hater liar loner widow evil fiend curse doom dread grief agony angst gory hex tarot bane
abort abuse acne coma cyst ulcer tumor polio palsy gout bile groin bowel femur feces fecal fetal fetus sinus
evict rehab detox keto diet opium hemp toxin toxic crypt cult creed rabbi imam vicar swami synod monk guru
yogi mecca druid piety cupid lisp deaf blind slur scum smear sham shun stash taint sully woe
"""
# Names, brands, places.
"""
maria peter mike rick roger jimmy john josh tom tony troy japan china roman welsh hogan yahoo smurf tron
jeep lotto ken don meg noel sally earl
"""
# Not words enough: slang, short for something, old or bookish.
"""
wanna gonna gimme dunno kinda jeez yep yup yea gosh blah aha aloha howdy ado alas thus hence oft doth thou
thy shalt ergo est quasi sans ids ifs ems pas mas pres gov sec alt cal gen app admin intel inter multi mono
meta mod rep rev sim info intro demo expo emo cant wont rigor tort libel penal renal lipid locus assay
axial basal amino aegis scion sire seer firth heath berg cabal duchy padre junta mogul czar posse twain
folio fiat chit fob ante anon onus nil ism cusp crux hove stead emery gabby lira viva tat tiff duff zeta
theta sigma iota nerdy jock hobo brad chino
"""
# Forms of other words: past tenses, plurals, comparisons.
"""
arose awoke bled broke chose clung crept dealt drank drawn drove fled flown flung froze gave gone got held
hid hung lent lit meant met ran risen said sang sat shook slept slid sped spent spoke spun stole stood
stung sung swam swept sworn swung took torn wept went woke woven dried cried felt lost sold found bound
geese mice women elves teeth axes fewer finer newer nicer wiser worst
"""
# Little words that make poor names.
"""
since quite maybe never now goes put away even ahead along among amid apart aside atop afar aback alike
alone aloof twice sure less redo
"""
# Found on reading the list through: rude, grown-up, odd or bookish.
"""
coon vulva wank wench bimbo vamp vixen twink randy moll pygmy thigh edema stent urea senna septa abed agape
amour amity anion anode anise argon boron axon auger awash axed batty bebop bicep bogey bunt burg busby
cadre canto chem comer conga cowl crick cull cumin cushy deuce dewy dilly dink dint emcee ensue ester exes
fed fife fugue gad gonzo gotta grad grist grog hep ibid iffy islet jib laity lire liege maven med nadir oxen
pooh posit prep ref rerun riven rusk scat sect shad shim shunt slit slung sank sump sunk swank swath swore
thine tithe twill vex vim wadi woken worse yip yule zed pip siege salvo loss sad
allay allot amass atone bide decry deter defy delve duly elegy elope exalt exude furor hark idly inane laud
liven nary ogle oust overt pall pert pithy quash rebus rebut remit scrip sever shirk spake spate tacit tenet
unify usurp wane wean wily wove aorta hymen ovary enema leper loin uric dike hick loony bozo prude smut crud
boozy hooch macho lusty miser mob cynic coke con gent unwed veep snafu bylaw dimer lisle borer covey coot
bevy bung shied stank dregs filth mange louse lousy dupe glut guano abhor ably abut afoul gaunt dinky
abet achy acrid afire ankh annul antic apace atria augur basso baud biddy bier bogie buxom calyx creel diff
dirge dross dweeb ennui fief gayer glob goner hake humus imbue ingot irk krone lithe nobly oases obit ovoid
ovum pica pith polyp poss quine rile scud sedge shod sibyl stunk swum tarry teat vapid venal wacko wuss
cagey dowdy woozy singe
aver baser bluer boor byway chide dacha daub divvy dote doyen espy extol fetid flout flunk foist gird gunny
hoary hovel hussy jeer julep kudzu lamer llano milch opine quire rearm redid ritzy sower surer swill tatty
tush wend klutz tilde blab
artsy bandy bigot circa dis eke exec fatal genus ire karma lupus lurid manic mace maxim olden pagan papal
patsy pew rte surly tardy ugh waldo eaten taken began safer fungi nexus triad servo
agog argot befit begot cabby coif conk dicta doff edify ewer filch gonad gyp halon hewer hokum inure jowl
loge mangy newsy odder oleo prig quoth roil scad schwa sidle sizer skulk spoor sward tromp wino yokel begat bruin ducal sager ashen zilch
beget chink pansy sop sod spunk lop non mid rump binge doer abate abide avail avert beset cease covet cower elude
evade incur infer inept ilk glib parse recur undid outdo utter ether ethic ethos dogma capon dado elfin
impel loamy mead nonce sago saner sis soppy telex waken cubit hooey tansy quaff seamy wader ague ecru exult gouty ovule sunup frump kapok umbel piker wist
""").split())

# Spellings that don't follow the sounds well enough.
HARD = re.compile(r"kn|wr|gn|mb$|ght|ough|ph|ps|eau|^rh|ei|aa|uu|ii|^x")

# Sounds easy to mistake for each other over the phone.
CONFUSED = [("P", "B"), ("T", "D"), ("K", "G"), ("S", "Z"), ("F", "V"), ("M", "N"), ("TH", "F"),
            ("TH", "DH"), ("F", "S"), ("CH", "SH"), ("JH", "ZH"), ("N", "NG"), ("IH", "IY"), ("EH", "AE"),
            ("AA", "AO"), ("UH", "UW"), ("AH", "AA")]


VOWELS = {"AA", "AE", "AH", "AO", "AW", "AY", "EH", "ER", "EY", "IH", "IY", "OW", "OY", "UH", "UW"}


def scowl(dir: Path, size: int) -> list[str]:
    return [l.strip() for l in (dir / f"wamerican.{size}").read_text(encoding="latin-1").splitlines()]


def cmu() -> dict[str, list[tuple[str, ...]]]:
    import cmudict

    out: dict[str, list[tuple[str, ...]]] = {}
    for word, prons in cmudict.dict().items():
        out[word] = [tuple(re.sub(r"\d", "", p) for p in pron) for pron in prons]
    return out


def near(a: tuple[str, ...], b: tuple[str, ...]) -> bool:
    """Whether `a` and `b` sound alike: one the other's start but for a
    sound or a few consonants more (car/card/cards, not car/carpet), or the
    same but for one confusable sound."""
    short, long = sorted((a, b), key=len)
    if long[:len(short)] == short:
        more = long[len(short):]
        return len(more) <= 1 or all(p not in VOWELS for p in more)
    if len(a) != len(b):
        return False
    diff = [(x, y) for x, y in zip(a, b) if x != y]
    return len(diff) == 1 and (diff[0] in CONFUSED or diff[0][::-1] in CONFUSED)


def base_form(w: str, known: set[str]) -> bool:
    """Whether `w` is an inflection of another word in `known`."""
    tries = []
    if w.endswith("s"):
        tries += [w[:-1]]
    if w.endswith("es"):
        tries += [w[:-2]]
    if w.endswith("ies"):
        tries += [w[:-3] + "y"]
    if w.endswith("ed"):
        tries += [w[:-1], w[:-2], w[:-3]]
    if w.endswith("ing"):
        return True
    if w.endswith("ly") or w.endswith("est"):
        tries += [w[:-2], w[:-3]]
    return any(len(t) >= 3 and t in known for t in tries)


def candidates(dir: Path, pron: dict) -> list[str]:
    import wordfreq

    small = scowl(dir, POOL)
    known = set(scowl(dir, 50))
    # Every common word's sounds, to find homophones (letters too: bee/b).
    common = {w for w in scowl(dir, 35) if re.fullmatch(r"[a-z]+", w)} | set("abcdefghijklmnopqrstuvwxyz")
    sounds: dict[tuple[str, ...], set[str]] = {}
    for w in common:
        for p in pron.get(w, []):
            sounds.setdefault(p, set()).add(w)
    out = []
    for w in small:
        if not re.fullmatch(r"[a-z]{3,5}", w) or not re.search(r"[aeiouy]", w):
            continue
        if w in STOP or w in BLOCK or HARD.search(w) or w not in pron or base_form(w, known):
            continue
        if any(len(sounds.get(p, set()) - {w}) for p in pron[w]):
            continue  # a homophone
        if len(pron[w]) > 1 and len(set(pron[w])) > 1 and not all(near(pron[w][0], p) for p in pron[w]):
            continue  # said two ways (read/read, wind/wind)
        out.append(w)
    out.sort(key=lambda w: (-wordfreq.zipf_frequency(w, "en"), w))
    return out


def pick(dir: Path) -> list[str]:
    pron = cmu()
    chosen: list[str] = []
    taken: list[tuple[str, ...]] = []
    for w in candidates(dir, pron):
        ps = pron[w]
        if any(near(p, t) for p in ps for t in taken):
            continue
        chosen.append(w)
        taken.extend(ps)
        if len(chosen) == COUNT:
            break
    if len(chosen) < COUNT:
        raise SystemExit(f"only {len(chosen)} words")
    return sorted(chosen)


def check(words: list[str]) -> list[str]:
    bad = []
    if len(words) != COUNT:
        bad.append(f"{len(words)} words, not {COUNT}")
    if len(set(words)) != len(words):
        bad.append("a word twice")
    for w in words:
        if not re.fullmatch(r"[a-z]{3,5}", w):
            bad.append(f"{w}: not 3 to 5 letters a-z")
        if w in BLOCK or w in STOP:
            bad.append(f"{w}: blocked")
    try:
        pron = cmu()
    except ImportError:
        return bad
    seen: dict[tuple[str, ...], str] = {}
    for w in words:
        for p in pron.get(w, []):
            for q, other in seen.items():
                if other != w and near(p, q):
                    bad.append(f"{w} sounds like {other}")
            seen[p] = w
    return bad


def main(argv: list[str]) -> None:
    if argv and argv[0] == "--check":
        words = [l.strip() for l in Path(argv[1]).read_text().splitlines() if l.strip() and not l.startswith("#")]
        bad = check(words)
        print("\n".join(bad) or f"{len(words)} words, all fine")
        sys.exit(1 if bad else 0)
    print("# Words for the network parts' random channel names (tools/opennet/words.py).")
    print("\n".join(pick(Path(argv[0]))))


if __name__ == "__main__":
    main(sys.argv[1:])
