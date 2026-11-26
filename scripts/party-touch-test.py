#!/usr/bin/env python3
# SPDX-License-Identifier: GPL-2.0-or-later
"""Native two-finger, pool drag, and save/resume regression on an isolated Android emulator.
Launch Games Fair with --locale en --env DAY_GAMES_SEED=15 first.
Usage: DAY_CLI=/path/to/day python3 scripts/party-touch-test.py emulator-SERIAL
Requires adb, javac, and an Android SDK with build-tools. Never targets physical devices.
The test resets only the three tested game rounds in the debug app on that emulator.
"""
import subprocess,json,time,os,xml.etree.ElementTree as ET,re
from pathlib import Path
import sys,shutil,tempfile,shlex
if len(sys.argv)!=2 or not sys.argv[1].startswith('emulator-'):
 raise SystemExit('Usage: party-touch-test.py emulator-SERIAL')
adb_bin=shutil.which('adb')
if not adb_bin: raise SystemExit('adb must be on PATH')
serial=sys.argv[1];root=str(Path(__file__).resolve().parents[1]);day=os.environ.get('DAY_CLI','day')
sdk=Path(os.environ.get('ANDROID_HOME') or os.environ.get('ANDROID_SDK_ROOT') or Path(adb_bin).resolve().parents[1])
android_jar=sorted(sdk.glob('platforms/android-*/android.jar'),key=lambda p:tuple(map(int,re.findall(r'\d+',p.parent.name))))[-1]
d8=sorted(sdk.glob('build-tools/*/d8'))[-1]
with tempfile.TemporaryDirectory(prefix='games-fair-touch-') as temp:
 subprocess.run(['javac','-source','8','-target','8','-classpath',str(android_jar),'-d',temp,str(Path(__file__).with_name('TwoFingers.java'))],check=True)
 jar=str(Path(temp)/'input.jar')
 subprocess.run([str(d8),'--lib',str(android_jar),'--output',jar,str(Path(temp)/'TwoFingers.class')],check=True)
 subprocess.run([adb_bin,'-s',serial,'push',jar,'/data/local/tmp/games-fair-input.jar'],check=True)

def adb(*a):return subprocess.check_output([adb_bin,'-s',serial,*a])
def drive(steps):
 p=subprocess.run([day,'drive','-t','android-material','--steps-json',json.dumps(steps)],cwd=root,env=dict(os.environ,ANDROID_SERIAL=serial),text=True,capture_output=True)
 if not p.stdout:raise RuntimeError(p.stderr)
 r=json.loads(p.stdout[p.stdout.index('{'):]);assert r['failed']==0,r;return r
def tap(id):return {'tap':{'id':id,'if_present':True}}
started=False
def start(route,prefix,index):
 global started
 if started:drive([{'nav_back':{}}])
 started=True
 drive([tap('home_players_'+str(index+1)),{'navigate':{'route':route}},tap(prefix+'_help_done'),tap(prefix+'_pause'),tap(prefix+'_new_game'),tap(prefix+'_change_players')])
def saved(key):
 adb('shell','input','keyevent','3');time.sleep(.3)
 xml=ET.fromstring(adb('exec-out','run-as','io.github.GamesFair','cat','shared_prefs/day_part_preferences.xml'))
 game=json.loads(next(n.text for n in xml if n.get('name')=='save.'+key))
 adb('shell','am','start','-n','io.github.GamesFair/dev.daybrite.day.bridge.DayActivity');time.sleep(.3)
 return game
def bounds(label):
 adb('shell','uiautomator','dump','/sdcard/games-fair-native.xml')
 xml=ET.fromstring(adb('exec-out','cat','/sdcard/games-fair-native.xml'))
 n=next(n for n in xml.iter('node') if n.get('content-desc','').startswith(label))
 return tuple(map(int,re.findall(r'\d+',n.get('bounds'))))
def viewport():
 x,y,r,b=bounds('Shared game arena.');s=min((r-x)/360,(b-y)/560)
 return x+(r-x-360*s)/2,y+(b-y-560*s)/2,s
start('paintfight','paintfight',3)
drive([{'pause':{'secs':3.2}},tap('paintfight_pause')]);ox,oy,s=viewport();drive([tap('paintfight_resume')])
adb('shell',f'CLASSPATH=/data/local/tmp/games-fair-input.jar app_process /system/bin TwoFingers {ox+306*s} {oy+506*s} {ox+54*s} {oy+54*s} {35*s}')
drive([tap('paintfight_pause')]);g=saved('paintfight.v1')
assert g['riders'][0]['pos']['x']<280 and g['riders'][1]['pos']['x']>80,g['riders']
print(json.dumps({'native_two_finger_steering':'passed','elapsed':g['elapsed']}),flush=True)
stopped=g['riders'];paint=g['paint']
drive([tap('paintfight_resume'),{'pause':{'secs':1}},tap('paintfight_pause')]);g=saved('paintfight.v1')
assert g['riders']==stopped and g['paint']==paint
print(json.dumps({'native_paint_release_stops':'passed'}),flush=True)
start('pool','pool',1);drive([tap('pool_pause')]);ox,oy,s=viewport();drive([tap('pool_resume')])
x,y=round(ox+180*s),round(oy+410*s)
adb('shell','input','swipe',str(x),str(y),str(x),str(round(y+95*s)),'800')
time.sleep(.25);drive([tap('pool_pause')]);g=saved('pool.v1')
assert g['moving'] and abs(g['balls'][0]['pos']['y']-410)>5,g
print(json.dumps({'native_pool_pullback':'passed','cue':g['balls'][0]['pos']}),flush=True)
drive([tap('pool_resume'),{'pause':{'secs':12}},tap('pool_pause')]);g=saved('pool.v1')
assert not g['moving'];assert any(abs(b['pos']['x']-180)>10 for b in g['balls'][1:]);print(json.dumps({'pool_settles_and_saves':'passed','turn':g['turn'],'scores':g['scores']}),flush=True)
start('moles','moles',3);drive([{'pause':{'secs':3.45}},tap('moles_pause')]);g=saved('moles.v1')
x,y,r,b=bounds('Game board.');density=int(re.findall(r'density: (\d+)',adb('shell','wm','density').decode())[-1])/160;c=min((r-x-12*density)/4,(b-y-12*density)/8,120*density);ox=x+(r-x-4*c)/2;oy=y+(b-y-8*c)/2
positions=[(.6,0),(2.8,.2),(1.7,1.15),(.25,2.2),(2.6,2.2),(1.6,3.3),(.1,4.4),(2.7,4.55),(1.75,5.6),(.45,6.7),(2.9,6.6)]
points=[(ox+(positions[g['cells'].index(owner)][0]+.5)*c,oy+(positions[g['cells'].index(owner)][1]+.5)*c) for owner in [1,2]]
drive([tap('moles_resume')]);adb('shell',f'CLASSPATH=/data/local/tmp/games-fair-input.jar app_process /system/bin TwoFingers {points[0][0]} {points[0][1]} {points[1][0]} {points[1][1]} 0')
drive([tap('moles_pause')]);g=saved('moles.v1');assert g['scores'][0]>=1 and g['scores'][1]>=1,g['scores'];print(json.dumps({'native_simultaneous_moles':'passed','scores':g['scores']}),flush=True)
