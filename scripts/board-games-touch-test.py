#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-or-later
"""Native regression for five board games on an isolated Android emulator.
Launch Games Fair with --locale en first. Resets only these emulator game rounds.
Usage: DAY_CLI=/path/to/day python3 scripts/board-games-touch-test.py emulator-SERIAL
Requires adb on PATH. Never targets physical devices.
"""
import subprocess,json,time,xml.etree.ElementTree as ET,re,os
from pathlib import Path
import shutil, sys
if len(sys.argv) != 2 or not sys.argv[1].startswith('emulator-'):
 raise SystemExit('Usage: board-games-touch-test.py emulator-SERIAL')
ADB=shutil.which('adb')
if not ADB: raise SystemExit('adb must be on PATH')
DAY=os.environ.get('DAY_CLI','day')
ROOT=str(Path(__file__).resolve().parents[1])
SERIAL=sys.argv[1]

def adb(*a):return subprocess.check_output([ADB,'-s',SERIAL,*a])
def start():
 session=next(s for s in json.loads(Path(ROOT+'/build/day/sessions.json').read_text()) if s['target']=='android-material')
 return adb('shell','am','start','-n','io.github.GamesFair/dev.daybrite.day.bridge.DayActivity','--es','day.env.DAYSCRIPT_PORT',str(session['enginePort']),'--es','day.env.DAYSCRIPT_TOKEN',session['engineToken'],'--es','day.env.DAY_GAMES_SEED','15')
def drive(steps):
 p=subprocess.run([DAY,'drive','-t','android-material','--steps-json',json.dumps(steps)],cwd=ROOT,env=dict(os.environ,ANDROID_SERIAL=SERIAL),capture_output=True,text=True)
 if p.returncode:raise RuntimeError(p.stderr[:1500]+p.stdout[:500])
 d=json.loads(p.stdout[p.stdout.index('{'):]);assert d['failed']==0,str(d)[:500];return d
def tap(id):return {'tap':{'id':id,'if_present':True}}
def wait(t):return {'pause':{'secs':t}}
def shot(id,a,b):return [{'touches':{'id':id,'phase':'began','points':[a]}},{'touches':{'id':id,'phase':'moved','points':[b]}},{'touches':{'id':id,'phase':'ended','points':[b]}}]
def state(game):
 adb('shell','input','keyevent','3');time.sleep(.4);xml=ET.fromstring(adb('exec-out','run-as','io.github.GamesFair','cat','shared_prefs/day_part_preferences.xml'));g=json.loads(next(n.text for n in xml if n.get('name')=='save.'+game+('.v1' if game=='dotsboxes' else '.v2')));start();time.sleep(.4);return g

def leave_game():
 drive([{'nav_back':{}},wait(.8),{'wait_for':{'id':'home'}}])

def open_game(name,n):
 drive([tap('home_players_'+str(n)),{'navigate':{'route':name}}, wait(.7), tap(name+'_help_done'),tap(name+'_pause'),tap('board_new_game'),wait(.8),{'screenshot':name+'-native'}])
def bounds(id):
 # The native accessibility tree clips to the navigation-bar viewport; the canvas itself
 # is edge-to-edge. Read the rendered canvas dimensions from its in-process capture.
 import struct
 name=id.removesuffix('_board')+'-native.png'
 data=Path(ROOT+'/build/day/screenshots/_drive/'+name).read_bytes()
 w,h=struct.unpack('>II',data[16:24]);return 0,0,w,h
# Android canvas inject coordinates are density-independent, measured by wm density.
density=float(re.search(rb'(\d+)\s*$',adb('shell','wm','density')).group(1))/160
adb('shell','am','force-stop','io.github.GamesFair')
start()
# Reconnect after the one cold launch before sending any taps. Keep the subsequent rounds
# in this process so a late response from a previous engine cannot consume the first tap.
for attempt in range(5):
 time.sleep(1)
 try:
  drive([{'wait_for':{'id':'home'}}])
  break
 except RuntimeError:
  if attempt==4: raise
open_game('minigolf',2);x,y,w,h=bounds('minigolf_board');w/=density;h/=density;k=min(w/9.75,h/21.2)
drive(shot('minigolf_board',[w/2,h/2+5*k],[w/2,h/2+8*k])+[wait(2),{'screenshot':'minigolf-native-scored'}]);g=state('minigolf');assert g['scores'][0]==1 and g['turn']==1,g;assert len(g['courses'])==9 and any(c['polygons']for c in g['courses']);print('Mini Golf scored; nine courses loaded',flush=True)
leave_game()
open_game('dotsboxes',2);x,y,w,h=bounds('dotsboxes_board');w/=density;h/=density;side=min(w*.866,h-200);ox=(w-side)/2;oy=(h-side)/2;c=side/5
for a in [[ox+c/2,oy],[ox,oy+c/2],[ox+c/2,oy+c],[ox+c,oy+c/2]]:drive(shot('dotsboxes_board',a,a)+[wait(1.5)])
g=state('dotsboxes');assert sum(g['scores'])==1 and g['turn']==1,g;print('Dots and Boxes captured a box and retained turn',flush=True)
leave_game()
open_game('buttonsoccer',2);g=state('buttonsoccer');assert len(g['formations'])==6;drive([tap('board_resume')]);x,y,w,h=bounds('buttonsoccer_board');w/=density;h/=density;k=min(w/21.5,h/47.5);b=g['bodies'][0]['p'];a=[w/2+b['x']*k,h/2-b['y']*k];z=[a[0],a[1]+4*k];drive(shot('buttonsoccer_board',a,z)+[wait(8),{'screenshot':'soccer-native-shot'}]);g=state('buttonsoccer');assert g['turn']==1 and not g['moving'],g;print('Button Soccer shot and turn settled; six formations loaded',flush=True)
leave_game()
open_game('fruitmerge',1);x,y,w,h=bounds('fruitmerge_board');w/=density;h/=density
for i in range(8):
 a=[w*(.3 if i%2==0 else .7),h*.55];drive(shot('fruitmerge_board',a,a)+[wait(1.1)])
g=state('fruitmerge');assert len(g['shapes'])==11;assert len(g['history'])==8;assert all(f['p']['y']<930 for f in g['fruits']),g
before=g['next'];drive([tap('board_command:0'),wait(.3)]);g=state('fruitmerge');assert len(g['history'])==7;print('Fruit Merge fruit outlines, drops, stacking and undo passed',flush=True)
drive([tap('board_resume'), wait(.3)])
# The stack keeps simulating. Capture its live frame without waiting for a quiet layout.
(Path(ROOT) / 'build/day/screenshots/_drive/fruit-native-stacked.png').write_bytes(
    adb('exec-out', 'screencap', '-p')
)
# Every player must release the private reveal before passing; confirm a full collective vote.
leave_game()
drive([tap('home_players_3'),{'navigate':{'route':'impostor'}}, wait(.7), tap('impostor_help_done'),tap('impostor_resume'),{'screenshot':'impostor-native-private'}]);g=state('impostor');assert g['count']==3 and len(g['word'])>0,g;drive([tap('impostor_resume')])
for i in range(3):drive([{'touches':{'id':'impostor_reveal','phase':'began','points':[[100,100]]}},wait(.5),{'touches':{'id':'impostor_reveal','phase':'ended','points':[[100,100]]}},tap('impostor_next'),wait(.3)])
drive([tap('impostor_discussed'),tap('impostor_vote:0'),tap('impostor_confirm'),{'screenshot':'impostor-native-result'}]);g=state('impostor');assert g['phase']=='Result' and sum(g['scores'])>0,g;print('Impostor private reveals, discussion, group vote and scoring passed',flush=True)
drive([tap('impostor_resume'),tap('impostor_pause'),tap('impostor_settings'),{'screenshot':'impostor-native-settings'},tap('impostor_settings_done')]);print('Impostor settings opened without borrow errors',flush=True)
