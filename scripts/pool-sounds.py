#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-or-later
"""Original Pool Foley, synthesized from damped material resonances (no borrowed samples).

A rounded phenolic-ball knock, felt-tip cue contact, rubber cushions and a leather/net
pocket. Three excitation strengths change both attack and upper partials; runtime gain
then follows the actual collision's normal velocity. PCM format is identical on all targets.
Rebuild: python3 scripts/pool-sounds.py
"""
from pathlib import Path
import math
import random
import struct
import wave

RATE = 44100
ROOT = Path(__file__).resolve().parents[1] / 'resource/assets/sounds/pool'
ROOT.mkdir(parents=True, exist_ok=True)


def knock(samples, rng, start, modes, amplitude, attack, noise_amount):
    """Short, inharmonic body resonances plus low-passed contact noise; no impulse spike."""
    offset = round(start * RATE)
    low = 0.0
    for i in range(min(len(samples)-offset, round(0.22*RATE))):
        t = i / RATE
        onset = 1.0 - math.exp(-t/attack)
        low += 0.16 * (rng.uniform(-1, 1) - low)
        value = sum(weight * math.sin(math.tau*frequency*t) * math.exp(-t/decay)
                    for frequency, weight, decay in modes)
        value += noise_amount * low * math.exp(-t/0.008)
        samples[offset+i] += amplitude * onset * value


def render(name, kind, strength=0.5):
    rng = random.Random(1234)  # Repeatable original assets, including the contact texture.
    samples = [0.0] * round((0.56 if kind == 'pocket' else 0.24) * RATE)
    if kind == 'ball':
        # Body carries the sound; the brief high partial grows only for firm contacts.
        modes = [(620, 0.60, 0.015), (1120, 0.34, 0.010),
                 (1860, 0.10+0.16*strength, 0.006), (2940, 0.07*strength, 0.003)]
        knock(samples, rng, 0, modes, 0.73, 0.0018-0.0007*strength, 0.19)
    elif kind == 'cue':
        modes = [(285, 0.36, 0.022), (590, 0.51, 0.016),
                 (1060, 0.28, 0.008), (2050, 0.06+0.10*strength, 0.004)]
        knock(samples, rng, 0, modes, 0.73, 0.0022-0.0008*strength, 0.27)
    elif kind == 'cushion':
        modes = [(170, 0.51, 0.026), (340, 0.43, 0.018), (690, 0.14*strength, 0.010)]
        knock(samples, rng, 0, modes, 0.81, 0.003, 0.35)
    else:
        # Quiet rim contact, then a low landing at presentation::LAND_AT and one settling knock.
        knock(samples, rng, 0.0, [(410,0.48,0.016),(760,0.18,0.009)],0.34,0.0025,0.32)
        knock(samples, rng, 0.28, [(155,0.54,0.042),(310,0.39,0.028),(640,0.16,0.014)],0.90,0.003,0.48)
        knock(samples, rng, 0.355, [(240,0.45,0.022),(530,0.21,0.014)],0.30,0.003,0.35)
    # Tiny table/room reflections add body, with no long reverberation or metallic ring.
    dry = samples.copy()
    for delay, gain in [(0.011, 0.10), (0.023, 0.045)]:
        shift = round(delay * RATE)
        for i in range(shift, len(samples)):
            samples[i] += dry[i-shift]*gain
    # DC block and fade to exact zero. Preserve level differences instead of peak normalizing.
    previous = high = 0.0
    for i, value in enumerate(samples):
        high = value-previous+0.994*high
        previous = value
        samples[i] = high * min(1.0, (len(samples)-1-i)/(0.012*RATE))
    peak = max(map(abs, samples))
    assert peak < 0.94, (name, peak)
    with wave.open(str(ROOT / (name+'.wav')), 'wb') as out:
        out.setparams((1, 2, RATE, len(samples), 'NONE', 'not compressed'))
        out.writeframes(b''.join(struct.pack('<h', round(x*32767)) for x in samples))
    print(f'{name}: {len(samples)/RATE:.2f}s, peak {peak:.3f}')


for material in ['ball', 'cue', 'cushion']:
    for label, strength in [('soft',0.12),('medium',0.5),('firm',1.0)]:
        render(f'{material}_{label}',material,strength)
render('pocket','pocket')
