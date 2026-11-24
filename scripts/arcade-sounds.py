#!/usr/bin/env python3
# SPDX-License-Identifier: AGPL-3.0-only WITH App-Fair-Distribution-Exception
"""Synthesize the shared-device game cues as mono 44.1 kHz PCM using the standard library."""
from array import array
import math
from pathlib import Path
import random
import sys
import wave

OUTPUT = Path(__file__).resolve().parents[1] / 'resource/assets/sounds'
RATE = 44100


def knock(pitch, decay, *, grit=.15, bend=.18, partial=2.67):
    """A damped pair of resonances with the short broadband attack of a struck surface."""
    def sample(t, noise):
        phase = math.tau*pitch*(t + bend*.022*(1-math.exp(-t/.022)))
        return (math.sin(phase) + .29*math.sin(partial*phase)*math.exp(-28*t))*math.exp(-t/decay) + grit*noise*math.exp(-t/.011)
    return sample


def brush(duration, frequency):
    """Filtered air with a rounded attack and a moving resonance."""
    def sample(t, noise):
        if t >= duration:
            return 0
        pulse = math.sin(math.pi*t/duration)**1.45
        return pulse*noise*(.55+.45*math.sin(math.tau*frequency*(t+1.3*t*t)))
    return sample


def chime(pitches, spacing=.065):
    return [(i*spacing, .82**i, knock(f,.071,grit=.013,bend=0,partial=2)) for i,f in enumerate(pitches)]


def write(name, layers):
    rng = random.Random(name)
    length = max(onset for onset, _, _ in layers)+.48
    samples = []
    filtered = 0
    for frame in range(math.ceil(length*RATE)):
        t = frame/RATE
        filtered = .55*filtered+.45*rng.uniform(-1,1)
        value = sum(gain*voice(t-onset,filtered)*min(1,(t-onset)*1200)
                    for onset,gain,voice in layers if t>onset)
        samples.append(value)
    # Keep a short tail, with a click-free release even for the lingering metal cues.
    end = max(i for i,s in enumerate(samples) if abs(s)>.001)+1
    samples = samples[:min(len(samples),end+200)]
    gain = .77/max(abs(s) for s in samples)
    pcm = array('h',(round(s*gain*32767*min(1,(len(samples)-i)/200)) for i,s in enumerate(samples)))
    if sys.byteorder != 'little':
        pcm.byteswap()
    path = OUTPUT/(name+'.wav')
    path.parent.mkdir(exist_ok=True,parents=True)
    with wave.open(str(path),'wb') as out:
        out.setparams((1,2,RATE,0,'NONE','not compressed'))
        out.writeframes(pcm.tobytes())


def main():
    cues = {
        'basketball/rim': [(0,1,knock(422,.064,partial=3.72)),(.014,.35,knock(720,.048))],
        'basketball/throw': [(0,1,brush(.145,620))],
        'basketball/spawn': chime([310,466],.036),
        'basketball/score': [(0,.9,brush(.21,780)),(.055,.19,knock(712,.042))],
        'basketball/turn': chime([370,494]), 'basketball/finish': chime([440,554,659,880]),
        'snakes/hiss': [(0,1,brush(.24,2400)),(.11,.38,brush(.17,1800))],
        'snakes/growth': chime([329,493,659],.034),
        'memory/flip': [(0,1,brush(.065,1050)),(.052,.5,knock(289,.017))],
        'memory/pair': chime([523,659],.055), 'memory/collect': chime([392,494,587],.042),
        'moles/hit': [(0,1,knock(151,.052,bend=.64)),(.012,.34,knock(375,.025))],
        'moles/miss': [(0,1,brush(.092,510))],
        'moles/dirt': [(i*.024,.8**i,knock(260+i*51,.012,grit=.95)) for i in range(5)],
        'dotsboxes/line': [(0,.6,brush(.065,960)),(.035,1,knock(450,.022))],
        'dotsboxes/box': chime([523,784],.048),
        'buttonsoccer/flick': [(0,.60,brush(.047,850)),(.008,1,knock(505,.021,grit=.32))],
        'buttonsoccer/disc': [(0,1,knock(535,.019,partial=2.8,grit=.28)),(.003,.46,knock(918,.014))],
        'buttonsoccer/kick': [(0,1,knock(147,.048,bend=.70,partial=1.86,grit=.39)),
                             (.005,.42,knock(280,.024)),(.009,.23,brush(.066,980))],
        'buttonsoccer/wall': [(0,1,knock(214,.038,partial=1.65,grit=.33)),(.008,.32,knock(346,.019))],
        'buttonsoccer/goal': [(0,.80,brush(.24,1100))]+chime([392,494,587,784],.085),
        'buttonsoccer/own_goal': [(0,.46,knock(140,.065))]+chime([466,370,294],.095),
        'minigolf/hole': [(0,.8,knock(310,.026)),(.054,.66,knock(185,.036)),(.112,.35,knock(147,.05))],
        'fruitmerge/drop': [(0,1,knock(190,.044,bend=.72))],
        'fruitmerge/merge': chime([466,587,698],.043),
        'fruitmerge/undo': chime([659,440],.047),
        'fruitmerge/sweep': [(0,1,brush(.28,1100)),(.16,.35,knock(523,.053))],
        'fruitmerge/collision': [(0,1,knock(134,.026,grit=.09))],
        'fruitduel/swoosh': [(.058,1,brush(.245,1420))],
        'fruitduel/cut': [(0,.75,brush(.08,2650)),(.024,1,knock(245,.032,grit=.55))],
        'fruitduel/clang': [(0,1,knock(695,.098,partial=2.73)),(.008,.5,knock(1040,.071,partial=3.82))],
    }
    for i in range(7):
        cues[f'basketball/bounce_{i}'] = [(0,1,knock(83+3*i,.074-.002*i,grit=.19,bend=.52,partial=1.82)),
                                          (.003,.24,knock(212+9*i,.032,grit=.26))]
    for name, layers in cues.items():
        write(name,layers)


if __name__ == '__main__':
    main()
