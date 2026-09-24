#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
"""Real Android pointer regression for Match Three (an API 33+ emulator).

Launch first:
  day launch -p android-mdc --android-device SERIAL --env DAY_GAMES_SEED=15 --locale en --keep-alive
Then:
  python3 scripts/matchthree-touch-test.py SERIAL

DayScript checks score/moves while adb keeps a finger down. The native board bounds come
from accessibility, so the test works at different screen sizes and densities. It verifies
exploration, cancellation, lift-to-commit, and tap input, leaving a fresh board for play.
"""
import json
import os
from pathlib import Path
import re
import shlex
import subprocess
import sys
import xml.etree.ElementTree as ET


def main():
    if len(sys.argv) != 2 or not sys.argv[1].startswith('emulator-'):
        raise SystemExit('Usage: matchthree-touch-test.py emulator-SERIAL')
    serial = sys.argv[1]
    project = Path(__file__).resolve().parents[1]
    day = shlex.split(os.environ.get('DAY_CLI', 'day'))

    def adb(*args):
        return subprocess.check_output(['adb', '-s', serial, *args])

    def drive(*steps):
        result = subprocess.run([*day, 'drive', '-p', 'android-mdc', '--steps-json', json.dumps(steps)],
                                cwd=project, capture_output=True, text=True, timeout=90)
        if result.returncode:
            raise AssertionError(result.stdout + result.stderr)
        parsed = json.loads(result.stdout)
        assert parsed['failed'] == 0, parsed

    def unchanged():
        drive({'assert_text': {'id': 'mt-score', 'text': '0'}},
              {'assert_text': {'id': 'mt-moves', 'text': '22'}},
              {'assert_missing': {'id': 'mt-result'}})

    drive({'navigate': {'route': 'matchthree'}}, {'tap': {'id': 'mt-map'}},
          {'tap': {'id': 'mt-stage-1'}}, {'pause': {'secs': .3}})
    unchanged()
    adb('shell', 'uiautomator', 'dump', '/sdcard/matchthree-ui.xml')
    root = ET.fromstring(adb('exec-out', 'cat', '/sdcard/matchthree-ui.xml'))
    node = next(n for n in root.iter('node')
                if n.attrib.get('content-desc', '').startswith('Match Three board.'))
    x, y, right, bottom = map(int, re.findall(r'\d+', node.attrib['bounds']))
    density = int(re.findall(r'density: (\d+)', adb('shell', 'wm', 'density').decode())[-1]) / 160
    side = min(min(right-x, bottom-y) - 24*density, 560*density)
    ox, oy, cell = x+(right-x-side)/2, y+(bottom-y-side)/2, side/7

    def at(i):
        return [str(round(ox+(i % 7+.5)*cell)), str(round(oy+(i // 7+.5)*cell))]

    def pointer(phase, i):
        adb('shell', 'input', 'touchscreen', 'motionevent', phase, *at(i))

    def shot(name):
        # Some multi-display emulators prefix screencap stdout with a warning. Strip only that
        # transport text; the PNG bytes themselves are untouched.
        png = adb('exec-out', 'screencap', '-p')
        signature = png.find(b'\x89PNG\r\n\x1a\n')
        assert signature >= 0, 'screencap returned no PNG'
        folder = project / 'build/day/screenshots/android-mdc/native-matchthree'
        folder.mkdir(parents=True, exist_ok=True)
        (folder / f'{name}.png').write_bytes(png[signature:])

    # 46 -> 47 wins seed 15, but it must not score while the finger is still held.
    pointer('DOWN', 46)
    try:
        pointer('MOVE', 47)
        unchanged()
        shot('held-right')
        pointer('MOVE', 39)
        unchanged()
        shot('held-up')
        pointer('MOVE', 46)
    finally:
        pointer('UP', 46)
    drive({'pause': {'secs': .3}})
    unchanged()
    print('PASS: right/up previews leave score and moves unchanged; returning home cancels.', flush=True)

    pointer('DOWN', 46)
    try:
        pointer('MOVE', 47)
        unchanged()
    finally:
        pointer('UP', 47)
    drive({'assert_text': {'id': 'mt-score', 'text': '12540', 'timeout_secs': 10}},
          {'assert_text': {'id': 'mt-moves', 'text': '21'}},
          {'wait_for': {'id': 'mt-result', 'timeout_secs': 30}},
          {'tap': {'id': 'mt-play-again'}}, {'pause': {'secs': .3}})
    print('PASS: lifting commits the preview exactly once.', flush=True)

    adb('shell', 'input', 'tap', *at(46))
    adb('shell', 'input', 'tap', *at(47))
    drive({'assert_text': {'id': 'mt-score', 'text': '12540', 'timeout_secs': 10}},
          {'assert_text': {'id': 'mt-moves', 'text': '21'}},
          {'wait_for': {'id': 'mt-result', 'timeout_secs': 30}},
          {'tap': {'id': 'mt-play-again'}})
    print('PASS: tap-to-swap still commits exactly once; fresh board ready.', flush=True)


if __name__ == '__main__':
    main()
