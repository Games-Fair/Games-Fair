#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-or-later
"""Render the arcade illustrations with librsvg and Pillow. Run from any directory."""
import io
import json
import math
import subprocess
from copy import deepcopy
import xml.etree.ElementTree as ET
from pathlib import Path

from PIL import Image

ROOT = Path(__file__).resolve().parents[1]
ASSETS = ROOT / 'resource/assets'
DRAWINGS = (ROOT / 'scripts/art/arcade.svg').read_text().split('</svg>')[0]
TEAMS = ['#ff6c63', '#35c5e3', '#92cd52', '#ffc54e']
INK = '#403541'


def use(name, color='white', transform=''):
    return f'<use xlink:href="#{name}" color="{color}" transform="{transform}"/>'


def render(body, size=256, view='0 0 100 100'):
    svg = DRAWINGS.replace('viewBox="0 0 100 100"', f'viewBox="{view}"') + body + '</svg>'
    result = subprocess.run(['rsvg-convert', '-w', str(size)], input=svg.encode(), capture_output=True, check=True)
    return Image.open(io.BytesIO(result.stdout)).convert('RGBA')


def save(name, image):
    path = ASSETS / name
    path.parent.mkdir(parents=True, exist_ok=True)
    image.save(path)


def vector_sprite(filename, element, view, dependencies=()):
    """Export a self-contained vector source, keeping currentColor for runtime team ink."""
    root = ET.fromstring(DRAWINGS + '</svg>')
    ns = 'http://www.w3.org/2000/svg'
    ET.register_namespace('', ns)
    svg = ET.Element(f'{{{ns}}}svg', {'viewBox': view})
    svg.append(ET.Comment(' SPDX-License-Identifier: GPL-2.0-or-later '))
    defs = ET.SubElement(svg, f'{{{ns}}}defs')
    for name in dependencies:
        defs.append(deepcopy(root.find(f".//*[@id='{name}']")))
    svg.append(deepcopy(root.find(f".//*[@id='{element}']")))
    path = ASSETS / filename
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(ET.tostring(svg, encoding='utf-8') + b'\n')


def atlas(name, drawings, cols, unit):
    sheet = Image.new('RGBA', (cols * unit, math.ceil(len(drawings) / cols) * unit))
    for i, body in enumerate(drawings):
        sheet.alpha_composite(render(body, unit), (i % cols * unit, i // cols * unit))
    save(name, sheet)


# Each fruit has a distinct outer contour and an interior detail that reads at small sizes.
OVAL = 'M50 7C74 7 94 26 94 52S77 93 50 93 6 76 6 52 25 7 50 7Z'
BERRY = 'M50 15C75 3 98 25 88 50Q77 80 52 93 24 81 12 52-1 17 30 13Z'
POME = 'M48 17l-8-12 11 5 10-7-1 14C85 20 96 38 92 62 87 89 69 95 48 94 21 93 7 78 7 56 6 35 22 18 48 17Z'
APPLE = 'M49 20C73 4 96 20 92 50S76 94 60 91Q50 86 40 92 17 95 9 60 0 20 27 16q12-3 22 4Z'
PEAR = 'M41 14Q46 4 56 9c11 3 9 20 18 31 32 40 10 54-24 54S1 78 13 57c7-14 19-21 28-43Z'
LEMON = 'M8 60Q-1 51 10 44C17 18 47 5 71 12q10 3 13 0 11-1 8 12C99 54 74 84 47 88Q22 94 8 60Z'
LEAF = '<path d="M48 20q-2-13 8-17m-5 12Q32 15 29 4q15-2 22 11Z" fill="#649756" stroke="#416849" stroke-width="2.4"/>'
PIPS = ''.join(f'<path d="M{x} {y}l-2 5" stroke="#ffe0a0" stroke-width="3" stroke-linecap="round"/>' for x,y in [(22,38),(38,52),(60,36),(75,46),(52,70),(29,65),(66,60)])
STRIPES = '<path d="M41 10q-32 35-14 73m25-74q-15 43 0 81m11-78q22 32 10 70" fill="none" stroke="#397e5a" opacity=".8" stroke-width="6"/>'
DIMPLES = '<g fill="#c46137" opacity=".7"><circle cx="68" cy="69" r="2.6"/><circle cx="75" cy="62" r="2"/><circle cx="60" cy="72" r="1.8"/></g>'
FRUITS = [
    ('cherry', OVAL, '#ff847b', '#b52b53', LEAF),
    ('strawberry', BERRY, '#ff7979', '#c52b51', LEAF + PIPS),
    ('blueberry', OVAL, '#a0acff', '#4b568d', '<path d="m49 21 9 5 9-3-4 9 2 7-10-2-8 6-1-9-7-3 8-3Z" fill="#51558b"/>'),
    ('apricot', OVAL, '#ffbd70', '#e77b53', '<path d="M58 16q28 27 8 61" fill="none" stroke="#cf7654" stroke-width="3"/>'),
    ('lemon', LEMON, '#fff089', '#e6ab42', DIMPLES),
    ('apple', APPLE, '#ff9180', '#b93652', LEAF),
    ('pear', PEAR, '#d9e67c', '#87aa42', LEAF),
    ('orange', OVAL, '#ffd083', '#ef833c', LEAF + DIMPLES),
    ('pomegranate', POME, '#f47b83', '#ad3253', DIMPLES),
    ('honeydew', OVAL, '#e5ec96', '#94be6d', '<path d="M69 19q12 29 5 52" fill="none" stroke="#88b965" stroke-width="4"/>'),
    ('watermelon', OVAL, '#addf83', '#579f62', STRIPES),
]


def fruit(i, core_only=False):
    name, outline, light, dark, detail = FRUITS[i]
    shading = f'<radialGradient id="skin-{name}" cx=".30" cy=".22" r=".86"><stop stop-color="{light}"/><stop offset="1" stop-color="{dark}"/></radialGradient>'
    skin = f'<path d="{outline}" fill="url(#skin-{name})" stroke="{INK}" stroke-width="3" stroke-linejoin="round"/>'
    sheen = '<path d="M23 42q3-12 12-16" stroke="white" opacity=".52" stroke-width="4.5" stroke-linecap="round" fill="none"/>'
    return shading + skin + ('' if core_only else detail + sheen)


def section(i):
    # A cut face seen straight on, with a separate dark rind and raised flesh.
    skin = ['#e68b37', '#bd4f66', '#71a35a'][i]
    pulp = ['#ffb752', '#ef8796', '#ef756f'][i]
    result = f'<circle cx="50" cy="51" r="43" fill="{skin}" stroke="{INK}" stroke-width="3"/><circle cx="50" cy="51" r="37" fill="{pulp}" stroke="#ffe4b0" stroke-width="3"/>'
    if i == 0:
        return result + ''.join(f'<path d="M50 47l0-30" transform="rotate({a} 50 51)" stroke="#ffe3a4" stroke-width="3"/>' for a in range(0, 360, 45))
    for row in range(-2, 3):
        for col in range(-2, 3):
            if row * row + col * col > 5 or (i == 2 and (row + col) % 2):
                continue
            x, y = 50 + col * 11 + row % 2 * 3, 51 + row * 12
            result += f'<ellipse cx="{x}" cy="{y}" rx="{4 if i==1 else 2.4}" ry="4" fill="{("#aa395d" if i==1 else "#583c48")}"/>'
    return result


def collision_outline(i):
    # Sample supporting points from the painted skin. Leaves and highlights have no mass.
    if i in (2, 3):
        return []
    mask = render(fruit(i, core_only=True), 100).getchannel('A')
    rim = [(x+0.5,y+0.5) for y in range(100) for x in range(100) if mask.getpixel((x,y)) > 127]
    points = []
    for j in range(24):
        a = math.tau*j/24
        x,y = max(rim, key=lambda p: p[0]*math.cos(a) + p[1]*math.sin(a))
        point = {'x': (x-50)/100, 'y': (y-50)/100}
        if point not in points:
            points.append(point)
    return points


def ball_sheet():
    # Orthographic sphere with seams in ball space and highlights in court space.
    size = 192
    sheet = Image.new('RGBA', (8*size, 3*size))
    for frame in range(24):
        a = math.tau*frame/24
        ca, sa = math.cos(a), math.sin(a)
        tile = Image.new('RGBA', (size,size))
        pixels = tile.load()
        for py in range(size):
            y = (py+.5-size/2)/(size*.47)
            for px in range(size):
                x = (px+.5-size/2)/(size*.47)
                rr = x*x+y*y
                if rr >= 1:
                    continue
                z = math.sqrt(1-rr)
                u, v = y*ca - z*sa, y*sa + z*ca
                seam = min(abs(u), abs(.90*x+.43*v), abs(abs(.43*x-.90*v)-(.58+.12*u*u)))
                relief = math.sin(120*x)*math.sin(120*u)*.025
                light = .60 + .43*max(0, .75*z-.36*x-.51*y) + relief
                color = (245,143,67) if seam>.023 else (81,54,49)
                pixels[px,py] = tuple(round(min(255,c*light)) for c in color)+(round(min(1,(1-math.sqrt(rr))*size*.47)*255),)
        sheet.alpha_composite(tile,(frame%8*size,frame//8*size))
    save('basketball/ball.png',sheet)


def place(body, x, y, size, angle=0):
    return f'<g transform="translate({x} {y}) rotate({angle} 50 50) scale({size})">{body}</g>'


def banner(name, sky, ground, pieces):
    scene = f'<rect width="144" height="100" fill="{sky}"/><circle cx="120" cy="14" r="35" fill="white" opacity=".10"/><path d="M0 76Q42 56 82 68t62-6v38H0Z" fill="{ground}"/>'
    save(name+'/banner.png', render(scene+''.join(pieces), 432, '0 0 144 100'))


def main():
    for mark in ('cross', 'ring'):
        vector_sprite(f'tictactoe/{mark}.svg', mark, '0 0 100 100')
    atlas('moles/moles.png', [use('hole')]+[use('mole',c) for c in TEAMS],5,192)
    atlas('paintfight/rollers.png', [use('roller',c) for c in TEAMS],4,128)
    faces = [fruit(i) for i in (1,2,4,6,10)] + [use(id_) for id_ in ('gift','flower','beach','star')] + [use('mole',TEAMS[0]),use('snake',TEAMS[2]),use('roller',TEAMS[1]),use('ring'),use('cross'),fruit(8),use('blade','#96d6e4')]
    atlas('memory/faces.png',faces,4,128)
    save('memory/back.png',render(use('back'),128))
    vector_sprite('snakes/head.svg', 'snake', '0 0 100 100')
    vector_sprite('snakes/body.svg', 'coil', '0 0 100 100')
    for name in ('star','selection'):
        save(f'buttonsoccer/{name}.png',render(use(name)))
    atlas('fruitmerge/fruits.png',[fruit(i) for i in range(11)],4,256)
    (ASSETS/'fruitmerge/colliders.json').write_text(json.dumps([collision_outline(i) for i in range(11)],indent=1)+'\n')
    duel = {FRUITS[i][0]:fruit(i) for i in (7,8,10)}
    for i,name in enumerate(('orange','pomegranate','watermelon')):
        duel[name+'_cut'] = section(i)
    duel.update({'present':use('gift'),'flower':use('flower'),'ball':use('beach'),'star':use('star')})
    vector_sprite('fruitduel/scimitar.svg', 'scimitar', '0 0 100 256', ('metal',))
    for name,body in duel.items():
        save('fruitduel/'+name+'.png',render(body))
    for name in ('watermelon','pomegranate','orange_cut','pomegranate_cut','watermelon_cut'):
        alpha = render(duel[name]).getchannel('A').point(lambda v:round(v*.22))
        shadow=Image.new('RGBA',alpha.size,'#172336')
        shadow.putalpha(alpha)
        save('fruitduel/'+('fruit' if name=='watermelon' else name)+'_shadow.png',shadow)
    banner('moles','#b7d595','#7aad71',[place(use('hole'),2,27,.78),place(use('mole',TEAMS[0]),7,11,.7),place(use('hole'),69,46,.72),place(use('mole',TEAMS[1]),73,28,.67)])
    banner('tictactoe','#4e6384','#3c516c',[place(use('cross'),6,9,.78,-9),place(use('ring'),68,20,.73)])
    banner('memory','#e0ae73','#b57a63',[place(use('back'),9,4,.85,-10),place(use('back'),58,8,.82,9),place(fruit(1),66,22,.52,9)])
    banner('fruitduel','#4c8290','#34586e',[use('scimitar',TEAMS[0],'translate(14 85) rotate(31) scale(.39) translate(-39 -208)'),place(fruit(10),44,11,.83),place(section(0),94,45,.42)])
    ball_sheet()


if __name__ == '__main__':
    main()
