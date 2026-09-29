#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
"""Original Match Three "Sea Stones" cues: pebbles clicking together as waves wash over them.
Synthesized from filtered noise and resonances; no recordings or external samples.
Rebuild: python3 scripts/matchthree-stone-sounds.py
"""
from pathlib import Path
import math, random, struct, wave
RATE = 44100
ROOT = Path(__file__).resolve().parents[1] / 'resource/assets/sounds/matchthree/stones'
ROOT.mkdir(parents=True, exist_ok=True)


def click(samples, rng, start, pitch, amp):
    """A pebble knocking another: a noise tick rung through a stone's short, bright resonance."""
    r = math.exp(-math.pi * pitch / 25 / RATE)  # bandwidth about pitch / 25: rings a few ms
    c = 2 * r * math.cos(2 * math.pi * pitch / RATE)
    y1 = y2 = 0.0
    at = int(start * RATE)
    for i in range(int(0.06 * RATE)):
        t = i / RATE
        x = rng.uniform(-1, 1) * math.exp(-t / 0.0012) if t < 0.004 else 0.0
        y = x + c * y1 - r * r * y2
        y2, y1 = y1, y
        # A little of the body's lower knock under the tick.
        body = 0.1 * math.sin(2 * math.pi * pitch * 0.47 * t) * math.exp(-t / 0.006)
        if at + i < len(samples):
            samples[at + i] += amp * (0.12 * y + body * min(1, t / 0.0005))


def wash(samples, rng, start, length, amp, fizz):
    """A wave running up the shingle and draining back: soft low noise that swells and ebbs,
    with pebbles rattling as the water pulls back through them."""
    low = hiss = 0.0
    at = int(start * RATE)
    n = int(length * RATE)
    for i in range(n):
        t = i / n
        # Rise over the first third, then a long drain.
        env = math.sin(min(1, t / 0.35) * math.pi / 2) ** 2 * (1 - max(0, t - 0.35) / 0.65) ** 1.5
        noise = rng.uniform(-1, 1)
        low += 0.035 * (noise - low)
        # The foam's hiss: noise with its lows taken out, softened a little.
        hiss += 0.3 * (noise - low - hiss)
        if at + i < len(samples):
            samples[at + i] += amp * env * (low * 2.4 + hiss * 0.35)
    for _ in range(fizz):
        t = 0.35 + rng.random() ** 0.7 * 0.6
        click(samples, rng, start + t * length, rng.uniform(2800, 5200), amp * rng.uniform(0.15, 0.4))


def render(name, seed, parts):
    rng = random.Random(seed)
    duration = max(p[1] + (p[2] if p[0] == 'wash' else 0.06) for p in parts) + 0.1
    samples = [0.0] * int(duration * RATE)
    for part in parts:
        if part[0] == 'click':
            _, start, pitch, amp = part
            click(samples, rng, start, pitch, amp)
        else:
            _, start, length, amp, fizz = part
            wash(samples, rng, start, length, amp, fizz)
    # Fade the last few milliseconds to an exact zero, then leave headroom: these are quiet cues.
    tail = int(0.02 * RATE)
    for i in range(tail):
        samples[-1 - i] *= i / tail
    peak = max(abs(s) for s in samples) or 1
    gain = 0.6 / peak
    with wave.open(str(ROOT / (name + '.wav')), 'wb') as wav:
        wav.setparams((1, 2, RATE, 0, 'NONE', 'not compressed'))
        wav.writeframes(b''.join(struct.pack('<h', int(s * gain * 32767)) for s in samples))


# ('click', start, pitch Hz, amplitude) or ('wash', start, length, amplitude, rattling pebbles)
SCORES = {
    'swap': [('click', 0, 2500, 0.9), ('click', 0.055, 2150, 0.6)],
    'bloom': [('click', 0, 2300, 0.7), ('click', 0.045, 2700, 0.5), ('click', 0.1, 2000, 0.45),
              ('wash', 0.02, 0.6, 0.18, 3)],
    'cascade': [('click', 0, 2400, 0.6), ('click', 0.05, 2900, 0.5), ('click', 0.09, 2100, 0.45),
                ('click', 0.15, 3200, 0.4), ('wash', 0.0, 1.0, 0.28, 8)],
    'magic': [('click', 0, 900, 0.9), ('click', 0.06, 2300, 0.45), ('click', 0.12, 2700, 0.4),
              ('wash', 0.0, 1.3, 0.34, 10)],
    'victory': [('click', 0, 2200, 0.5), ('click', 0.12, 2600, 0.45), ('click', 0.24, 3000, 0.4),
                ('wash', 0.1, 2.8, 0.45, 26)],
}
for seed, (name, parts) in enumerate(SCORES.items()):
    render(name, seed + 7, parts)
