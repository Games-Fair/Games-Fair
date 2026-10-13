#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
"""Pac-Pac's cues, synthesized in the voice of the arcade's three-channel wavetable chip:
short tables of 32 four-bit samples, swept in pitch, with no recordings or external samples.
Rebuild: python3 scripts/pacpac-sounds.py

The looping clips (the sirens, the fright warble, the eyes' retreat) are each exactly one
second of whole cycles with a short fade at both ends, so the game can start one again just
before the last ends and the join is seamless.
"""
from pathlib import Path
import math, struct, wave

RATE = 44100
ROOT = Path(__file__).resolve().parents[1] / 'resource/assets/sounds/pacpac'
ROOT.mkdir(parents=True, exist_ok=True)

# Wavetables, 32 steps of 0..15, in the chip's idiom: a hollow square-ish tone, a sine, a
# saw with harmonics, and a buzzy table for the chomp.
SINE = [round(7.5 + 7.5 * math.sin(2 * math.pi * i / 32)) for i in range(32)]
HOLLOW = [15 if (i % 16) < 6 else (4 if (i % 16) < 8 else 0) for i in range(32)]
SAW = [round(15 * (i % 32) / 31) for i in range(32)]
BUZZ = [round(7.5 + 7.5 * math.sin(2 * math.pi * i / 32) * (1 if i % 4 < 2 else 0.4)) for i in range(32)]
TRI = [round(15 * (1 - abs(i - 16) / 16)) for i in range(32)]


def table_sample(table, phase):
    """The table read at `phase` (0..1), unsmoothed like the chip."""
    return (table[int(phase * 32) % 32] - 7.5) / 7.5


def render(seconds, voices):
    """Mix `voices`, each a function of time returning (frequency, amplitude, table), with
    the chip's stepping, into one float buffer."""
    n = int(seconds * RATE)
    out = [0.0] * n
    phases = [0.0] * len(voices)
    for i in range(n):
        t = i / RATE
        for v, voice in enumerate(voices):
            freq, amp, table = voice(t)
            if amp <= 0 or freq <= 0:
                continue
            phases[v] = (phases[v] + freq / RATE) % 1.0
            out[i] += amp * table_sample(table, phases[v])
    return out


def write(name, samples, peak=0.85, fade=0.004):
    n = len(samples)
    f = int(fade * RATE)
    for i in range(min(f, n)):
        g = i / f
        samples[i] *= g
        samples[n - 1 - i] *= g
    top = max(abs(s) for s in samples) or 1.0
    gain = peak / top
    with wave.open(str(ROOT / (name + '.wav')), 'wb') as w:
        w.setparams((1, 2, RATE, 0, 'NONE', 'not compressed'))
        w.writeframes(b''.join(struct.pack('<h', int(max(-1, min(1, s * gain)) * 32767)) for s in samples))


def sweep(lo, hi, cycles, table, amp=0.5, shape='tri', rise=0.5):
    """A one-second loop: `cycles` rises and falls between `lo` and `hi` Hz. `rise` is the
    fraction of each cycle spent climbing when the shape is a triangle."""
    def voice(t):
        u = (t * cycles) % 1.0
        if shape == 'tri':
            k = u / rise if u < rise else (1 - u) / (1 - rise)
        elif shape == 'rise':
            k = u
        else:
            k = (1 - math.cos(2 * math.pi * u)) / 2
        return lo + (hi - lo) * k, amp, table
    return voice


def note(freq, start, length, table, amp=0.5, decay=6.0, glide=0.0):
    """One note starting at `start`: a plucked envelope, with an optional glide of the pitch."""
    def voice(t):
        if t < start or t >= start + length:
            return 0, 0, table
        u = (t - start) / length
        env = min(1, (t - start) / 0.004) * math.exp(-decay * u) * min(1, (length - (t - start)) / 0.01)
        return freq * (1 + glide * u), amp * env, table
    return voice


def midi(n):
    return 440 * 2 ** ((n - 69) / 12)


# --- Pac-Pac --------------------------------------------------------------------------

# The sirens: the arcade's is its smooth sine-like table gliding up and back down, two
# cycles a second to begin with, a little quicker and higher each time the dots thin out.
# It sits well under the effects, so these peak low.
for level, (lo, hi, cycles) in enumerate([(400, 560, 2), (440, 620, 2), (480, 700, 3), (540, 780, 3), (600, 880, 4)], 1):
    write(f'siren_{level}', render(1.0, [sweep(lo, hi, cycles, SINE, 0.6, 'tri', 0.55)]), peak=0.5, fade=0.02)

# Frightened ghosts: a quick wobbling rise, over and over.
write('fright', render(1.0, [sweep(240, 900, 8, TRI, 0.55, 'rise')]), peak=0.5, fade=0.02)

# Eyes going home: a fast, thin chirp that keeps rising.
write('eyes', render(1.0, [sweep(900, 1900, 10, SINE, 0.45, 'rise')]), peak=0.45, fade=0.02)

# The chomp, in two halves that alternate: a dip and a rise through the buzzy table.
def chomp(name, down):
    def voice(t):
        u = t / 0.11
        if u >= 1:
            return 0, 0, BUZZ
        k = (1 - u) if down else u
        return 180 + 540 * k, 0.7 * (1 - 0.3 * u), BUZZ
    write(name, render(0.12, [voice]), peak=0.7)
chomp('chomp_a', True)
chomp('chomp_b', False)

# Eating a ghost: a long rising zip.
write('eat_ghost', render(0.5, [lambda t: (300 + 2200 * (t / 0.5) ** 1.6, 0.6 * (1 - t / 0.5) ** 0.3, SAW)]), peak=0.8)

# Eating fruit: a two-note bounce.
write('eat_fruit', render(0.45, [note(midi(76), 0.0, 0.18, TRI, 0.6, 4), note(midi(88), 0.17, 0.25, TRI, 0.55, 5)]), peak=0.8)

# Extra life: three bright chirps.
write('extra_life', render(1.0, [note(midi(84), 0.0 + i * 0.3, 0.22, SINE, 0.6, 3, 0.5) for i in range(3)]), peak=0.85)

# Death: a shrinking wail in five steps, then two pops.
def death_voice(t):
    if t < 1.1:
        step = int(t / 0.22)
        u = (t % 0.22) / 0.22
        base = 650 - 90 * step
        return base * (1 - 0.45 * u), 0.6, HOLLOW
    if 1.2 <= t < 1.3 or 1.4 <= t < 1.5:
        u = ((t - 1.2) % 0.2) / 0.1
        return 120 + 500 * u, 0.6, BUZZ
    return 0, 0, HOLLOW
write('death', render(1.55, [death_voice]), peak=0.85)

# The start of a game: the jingle, two voices.
melody = [71, 83, 78, 75, 83, 78, 75, 72, 84, 79, 76, 84, 79, 76,
          71, 83, 78, 75, 83, 78, 75, 75, 76, 77, 77, 78, 79, 79, 80, 81, 83]
beats = [1, 1, 1, 1, 1, 2, 2, 1, 1, 1, 1, 1, 2, 2,
         1, 1, 1, 1, 1, 2, 2, 1, 1, 1, 1, 1, 1, 1, 1, 1, 4]
bass = [59, 66, 59, 66, 59, 66, 59, 66, 60, 67, 60, 67, 60, 67, 60, 67,
        59, 66, 59, 66, 59, 66, 59, 66, 63, 64, 65, 66, 67, 68, 69, 71]
unit = 0.125
voices = []
t = 0.0
for n, b in zip(melody, beats):
    voices.append(note(midi(n), t, unit * b * 0.95, TRI, 0.5, 2.5))
    t += unit * b
tb = 0.0
for n in bass:
    voices.append(note(midi(n), tb, unit * 1.9, SAW, 0.28, 3))
    tb += unit * 2
    if tb > t:
        break
write('intro', render(t + 0.2, voices), peak=0.85)

# --- Ms. Pac-Pac ----------------------------------------------------------------------

# Her sirens: a lower, quicker wobble on the same smooth table.
for level, (lo, hi, cycles) in enumerate([(300, 420, 3), (340, 480, 3), (390, 560, 4), (450, 650, 4), (520, 760, 5)], 1):
    write(f'ms_siren_{level}', render(1.0, [sweep(lo, hi, cycles, SINE, 0.6, 'tri', 0.5)]), peak=0.5, fade=0.02)

# Her fright: a bubbling descending loop.
write('ms_fright', render(1.0, [sweep(900, 300, 6, TRI, 0.55, 'rise')]), peak=0.5, fade=0.02)

# Her chomp: one short rising tweet.
write('ms_chomp', render(0.09, [lambda t: (500 + 900 * t / 0.09, 0.65 * (1 - t / 0.09) ** 0.5, SINE)]), peak=0.7)

# Her death: a bouncing fall.
def ms_death_voice(t):
    bounces = [(0.0, 0.35, 700), (0.4, 0.3, 560), (0.75, 0.25, 440), (1.05, 0.2, 340)]
    for start, length, top in bounces:
        if start <= t < start + length:
            u = (t - start) / length
            return top * (1 - 0.6 * u), 0.6 * (1 - u * 0.5), TRI
    return 0, 0, TRI
write('ms_death', render(1.3, [ms_death_voice]), peak=0.85)

# Her jingle: a brighter theme with a walking bass.
ms_melody = [76, 79, 83, 79, 76, 72, 76, 79, 76, 79, 83, 79, 76, 72, 76, 79,
             77, 81, 84, 81, 77, 74, 77, 81, 79, 83, 86, 83, 79, 76, 79, 83, 88]
ms_beats = [1] * 32 + [4]
ms_bass = [48, 55, 52, 55, 48, 55, 52, 55, 50, 57, 53, 57, 52, 59, 55, 59, 48]
voices = []
t = 0.0
for n, b in zip(ms_melody, ms_beats):
    voices.append(note(midi(n), t, unit * b * 0.9, SINE, 0.5, 3))
    t += unit * b
tb = 0.0
for n in ms_bass:
    voices.append(note(midi(n), tb, unit * 1.8, SAW, 0.26, 3))
    tb += unit * 2
write('ms_intro', render(t + 0.2, voices), peak=0.85)

print('wrote', len(list(ROOT.glob('*.wav'))), 'clips to', ROOT)
