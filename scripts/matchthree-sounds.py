#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
"""Original Match Three glass-marimba cues. No recordings or external samples.
Rebuild: python3 scripts/matchthree-sounds.py
"""
from pathlib import Path
import math, struct, wave
RATE = 44100
ROOT = Path(__file__).resolve().parents[1] / 'resource/assets/sounds/matchthree'
ROOT.mkdir(parents=True, exist_ok=True)
# start (seconds), MIDI pitch, length, amplitude
SCORES = {
    'swap': [(0, 79, .11, .4), (.045, 84, .13, .3)],
    'bloom': [(0, 72, .27, .35), (.045, 76, .25, .3), (.09, 79, .32, .28)],
    'cascade': [(0, 79, .30, .3), (.065, 84, .32, .3), (.13, 88, .34, .25), (.2, 91, .4, .23)],
    'magic': [(0, 48, .42, .35), (.025, 72, .4, .25), (.08, 79, .42, .25), (.14, 84, .44, .23), (.2, 88, .5, .22)],
    'victory': [(0, 72, .4, .28), (.12, 76, .4, .26), (.24, 79, .4, .26), (.4, 84, .75, .24), (.4, 88, .75, .2), (.4, 91, .75, .16)],
}
for name, notes in SCORES.items():
    duration = max(t + length for t, _, length, _ in notes) + .16
    samples = [0.0] * int(duration * RATE)
    for start, pitch, length, amplitude in notes:
        frequency = 440 * 2 ** ((pitch - 69) / 12)
        for echo, gain in [(0, 1), (.073, .15), (.137, .07)]:
            for i in range(int(length * RATE)):
                t = i / RATE
                # Soft attack, decaying glass harmonics, and an exact zero tail prevent clicks.
                env = min(1, t / .004) * math.exp(-5 * t / length) * min(1, (length - t) / .025)
                tone = math.sin(2 * math.pi * frequency * t) + .25 * math.sin(2 * math.pi * frequency * 2.003 * t) * math.exp(-15 * t) + .08 * math.sin(2 * math.pi * frequency * 4.01 * t) * math.exp(-30 * t)
                at = int((start + echo) * RATE) + i
                if at < len(samples): samples[at] += amplitude * gain * env * tone
    peak = max(abs(s) for s in samples)
    gain = min(1, .88 / peak)
    with wave.open(str(ROOT / (name + '.wav')), 'wb') as wav:
        wav.setparams((1, 2, RATE, 0, 'NONE', 'not compressed'))
        wav.writeframes(b''.join(struct.pack('<h', int(s * gain * 32767)) for s in samples))
