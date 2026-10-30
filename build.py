"""Builds the mod from an installed copy of Armored Core VI 1.09.1.

    python build.py --game "<...>/ARMORED CORE VI FIRES OF RUBICON/Game" --witchy "<...>/WitchyBND.exe"

Needs git, node/npm, cargo (stable, MSVC) and WitchyBND 3.x. Downloads doomgeneric and the shareware DOOM1.WAD on
the first run. Writes package/: the me3 profile, the mod folder, the DLL and doom/ with the WAD.
"""
import argparse
import hashlib
import os
import re
import shutil
import struct
import subprocess
import urllib.request
import xml.etree.ElementTree as ET

ROOT = os.path.dirname(os.path.abspath(__file__))
WORK = os.path.join(ROOT, "build")
PACKAGE = os.path.join(ROOT, "package")

DOOMGENERIC_URL = "https://github.com/ozkl/doomgeneric"
DOOMGENERIC_COMMIT = "e257a09527062bfb986b6e5e0780839b6cbd5b71"
WAD_URL = "https://raw.githubusercontent.com/Akbar30Bill/DOOM_wads/master/doom1.wad"
WAD_SHA1 = "5b2e249b9c5133ec987b3ea77596381dc0d6bc1d"  # shareware DOOM1.WAD v1.9

TOP_ID, BOTTOM_ID = 9990001, 9990002  # the two screen halves, must match fxr/build_screen.mjs
TURNER_BULLET, SCUDDER_BULLET = 10110000, 10120000
# Both bullets hang in place for a day, each carrying one half of the screen.
BULLET_EDITS = {"life": 86400.0, "dist": 100000.0, "initVellocity": 0.01, "maxVellocity": 0.01, "minVellocity": 0.0}

FIELD_SIZES = {'s8': 1, 'u8': 1, 'dummy8': 1, 's16': 2, 'u16': 2, 's32': 4, 'u32': 4, 'f32': 4, 'angle32': 4,
               'b32': 4, 'f64': 8, 'fixstr': 1, 'fixstrW': 2}
FIELD_FORMATS = {'s32': '<i', 'f32': '<f'}


class Param:
    """Raw .param file, fields located through WitchyBND's Paramdex defs, edited in place."""

    def __init__(self, path, def_path):
        self.path = path
        self.data = bytearray(open(path, "rb").read())
        self.fields, offset, bit_size, bits_used = {}, 0, None, 0
        for field in ET.parse(def_path).getroot().iter("Field"):
            typ, name, bits, count = re.match(r"(\w+)\s+(\w+)(?::(\d+))?(?:\[(\d+)\])?", field.get("Def")).groups()
            size = FIELD_SIZES[typ]
            if bits:  # bitfields share a storage unit until it's full
                if bit_size == size and bits_used + int(bits) <= size * 8:
                    bits_used += int(bits)
                    continue
                bit_size, bits_used = size, int(bits)
                offset += size
                continue
            bit_size = None
            self.fields[name] = (offset, typ)
            offset += size * (int(count) if count else 1)
        row_count = struct.unpack_from("<H", self.data, 0xA)[0]
        self.rows = dict(struct.unpack_from("<i4xQ", self.data, 0x40 + i * 0x18) for i in range(row_count))

    def set(self, row, field, value):
        offset, typ = self.fields[field]
        struct.pack_into(FIELD_FORMATS[typ], self.data, self.rows[row] + offset, value)

    def save(self):
        open(self.path, "wb").write(self.data)


def unpacked(path):
    # WitchyBND's folder naming: regulation.bin -> regulation-bin, x.ffxbnd.dcx -> x-ffxbnd-dcx-wffxbnd
    name = os.path.basename(path)
    return os.path.join(WORK, name.replace(".", "-") + ("-wffxbnd" if name.endswith(".ffxbnd.dcx") else ""))


def fetch_dependencies():
    doomgeneric = os.path.join(ROOT, "dll", "doomgeneric")
    if not os.path.isdir(doomgeneric):
        subprocess.run(["git", "clone", DOOMGENERIC_URL, doomgeneric], check=True)
        subprocess.run(["git", "checkout", DOOMGENERIC_COMMIT], cwd=doomgeneric, check=True)
    wad = os.path.join(ROOT, "wad", "doom1.wad")
    if not os.path.exists(wad):
        os.makedirs(os.path.dirname(wad), exist_ok=True)
        data = urllib.request.urlopen(WAD_URL).read()
        assert hashlib.sha1(data).hexdigest() == WAD_SHA1, "downloaded DOOM1.WAD has the wrong hash"
        open(wad, "wb").write(data)


def main():
    parser = argparse.ArgumentParser(description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter)
    parser.add_argument("--game", required=True, help="the game's Game folder (with regulation.bin and sfx/)")
    parser.add_argument("--witchy", required=True, help="WitchyBND.exe (3.x)")
    args = parser.parse_args()
    defs = os.path.join(os.path.dirname(args.witchy), "Assets", "Paramdex", "AC6", "Defs")
    fetch_dependencies()

    # WitchyBND exits 0 even when it does nothing, so check the results.
    def unpack(path):
        subprocess.run([args.witchy, "-s", path], check=True)
        assert os.path.isdir(unpacked(path)), f"WitchyBND didn't unpack {path}"

    def repack(path):
        before = os.path.getmtime(path)
        subprocess.run([args.witchy, "-s", unpacked(path)], check=True)
        assert os.path.getmtime(path) != before, f"WitchyBND didn't repack {path}"

    shutil.rmtree(WORK, ignore_errors=True)
    os.makedirs(WORK)
    regulation = shutil.copy(os.path.join(args.game, "regulation.bin"), WORK)
    effects = shutil.copy(os.path.join(args.game, "sfx", "sfxbnd_commoneffects.ffxbnd.dcx"), WORK)
    unpack(regulation)
    unpack(effects)

    # The screen: two FXRs. They only use a built-in texture, but WitchyBND would generate empty resource lists for
    # them, so they get a stock bullet effect's.
    fxr_dir = os.path.join(ROOT, "fxr")
    shell = os.name == "nt"
    if not os.path.isdir(os.path.join(fxr_dir, "node_modules")):
        subprocess.run(["npm", "install"], cwd=fxr_dir, check=True, shell=shell)
    subprocess.run(["node", "build_screen.mjs", os.path.join(unpacked(effects), "effect")], cwd=fxr_dir, check=True)
    reslists = os.path.join(unpacked(effects), "ResourceList")
    for fxr_id in (TOP_ID, BOTTOM_ID):
        shutil.copy(os.path.join(reslists, "f000300031.ffxreslist"), os.path.join(reslists, f"f{fxr_id:09d}.ffxreslist"))

    bullets = Param(os.path.join(unpacked(regulation), "Bullet.param"), os.path.join(defs, "BulletParam.xml"))
    for row, fxr_id in ((TURNER_BULLET, TOP_ID), (SCUDDER_BULLET, BOTTOM_ID)):
        bullets.set(row, "sfxId_Bullet", fxr_id)
        for field, value in BULLET_EDITS.items():
            bullets.set(row, field, value)
    bullets.save()

    repack(regulation)
    repack(effects)
    subprocess.run(["cargo", "+stable", "build", "--release"], cwd=os.path.join(ROOT, "dll"), check=True)

    shutil.rmtree(PACKAGE, ignore_errors=True)
    os.makedirs(os.path.join(PACKAGE, "mod", "sfx"))
    os.makedirs(os.path.join(PACKAGE, "doom"))
    shutil.copy(regulation, os.path.join(PACKAGE, "mod"))
    shutil.copy(effects, os.path.join(PACKAGE, "mod", "sfx"))
    shutil.copy(os.path.join(ROOT, "dll", "target", "release", "fxr_doom.dll"), PACKAGE)
    shutil.copy(os.path.join(ROOT, "fxr-doom.me3"), PACKAGE)
    # The DLL plays the first of doom2.wad, doom.wad, doom1.wad it finds in doom/.
    for wad in os.listdir(os.path.join(ROOT, "wad")):
        if wad.lower().endswith(".wad"):
            shutil.copy(os.path.join(ROOT, "wad", wad), os.path.join(PACKAGE, "doom", wad.lower()))
    shutil.copy(os.path.join(ROOT, "dll", "doomgeneric", "LICENSE"), os.path.join(PACKAGE, "doom", "doomgeneric-LICENSE.txt"))
    print(f"Built {PACKAGE}")


if __name__ == "__main__":
    main()
