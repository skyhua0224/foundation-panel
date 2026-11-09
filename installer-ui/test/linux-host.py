#!/usr/bin/env python3
"""Scripted stand-in for the installer shell on Linux: runs the setup UI (webkit2gtk build of installer-ui) under
Xvfb on pipes, answers its actions like tools/installer_web.inc and saves a screenshot after every view.
usage: linux-host.py <ui-exe> <out-dir>"""
import json, os, subprocess, sys, threading, time, copy

ui, out = sys.argv[1], sys.argv[2]
os.makedirs(out, exist_ok=True)
disp = ':5'
xvfb = subprocess.Popen(['broadwayd', disp], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
time.sleep(1.5)
COMP = ['vdd', 'gamepad', 'usbip', 'vmouse', 'firewall', 'autostart', 'tools', 'desktopicon']
GROUPS = [('prepare', ['worker', 'engine', 'wizard', 'prepare']), ('files', ['files', 'path', 'config', 'permissions']),
          ('drivers', ['vdd', 'gamepad', 'usbip', 'vmouse']), ('system', ['firewall', 'service', 'autostart', 'reboot'])]
v = {'schema': 1, 'page': 'maintenance', 'locale': 'zh', 'upgrade': True,
     'maintenance': {'installedVersion': '2026.1101.0', 'packageVersion': '2026.1109.0', 'downgrade': False, 'same': False, 'hasUninstaller': True},
     'components': {c: c != 'vmouse' for c in COMP}, 'allSelected': False, 'uacCancelled': False,
     'progress': {'groups': [{'id': g, 'state': 'pending'} for g, _ in GROUPS], 'current': 0, 'steps': {}, 'lastStep': None, 'finished': False},
     'restart': 'none', 'rebootReasons': [], 'failure': None, 'exitCode': 0, 'runId': 'linux-1', 'guidePending': False,
     'opening': False, 'panelFailed': False, 'busy': False}
env = dict(os.environ, GDK_BACKEND='broadway', BROADWAY_DISPLAY=disp, FOUNDATION_SETUP_UI_SHOTS=out + '/rust',
           FOUNDATION_SETUP_UI_DRIVE='customize,toggle-component:usbip,back,install,' + 'noop,' * 8 + 'open-panel,noop,exit',
           WEBKIT_DISABLE_COMPOSITING_MODE='1')
p = subprocess.Popen([ui], stdin=subprocess.PIPE, stdout=subprocess.PIPE, env=env, text=True, bufsize=1)
log = open(out + '/protocol.log', 'w')
seq = 0
lock = threading.Lock()

def push():
    global seq
    with lock:
        seq += 1
        v['seq'] = seq
        v['allSelected'] = all(v['components'].values())
        v['busy'] = v['page'] in ('uac', 'installing')
        line = json.dumps({'type': 'view', 'view': v}, ensure_ascii=False)
        log.write('> view %d %s\n' % (seq, v['page'])); log.flush()
        try:
            p.stdin.write(line + '\n'); p.stdin.flush()
        except BrokenPipeError:
            return
        n, page = seq, v['page']
    def shot():
        time.sleep(0.6)
        subprocess.run(['chromium', '--headless=new', '--no-sandbox', '--disable-gpu', '--window-size=1060,760', '--virtual-time-budget=2500',
                        '--screenshot=%s/%02d-%s.png' % (out, n, page), 'http://127.0.0.1:8085/'], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL, timeout=40)
    threading.Thread(target=shot, daemon=True).start()

def run_install():
    v['page'] = 'uac'; push(); time.sleep(1.5)
    steps = [('worker', 'running'), ('engine', 'running'), ('files', 'running'), ('files', 'done'), ('vdd', 'running'), ('vdd', 'done'), ('service', 'running')]
    for st, state in steps:
        v['page'] = 'installing'
        v['progress']['steps'][st] = state
        v['progress']['lastStep'] = st
        gi = next(i for i, (g, s) in enumerate(GROUPS) if st in s)
        v['progress']['current'] = gi
        for i, (g, s) in enumerate(GROUPS):
            seen = [v['progress']['steps'][x] for x in s if x in v['progress']['steps']]
            v['progress']['groups'][i]['state'] = 'pending' if not seen else ('running' if i == gi else 'done')
        push(); time.sleep(1.5)
    v['progress']['finished'] = True
    for gr in v['progress']['groups']: gr['state'] = 'done'
    v['page'] = 'done'; push()

push()
for line in p.stdout:
    line = line.strip()
    log.write('< ' + line + '\n'); log.flush()
    try: m = json.loads(line)
    except Exception: continue
    if m.get('type') != 'action': continue
    a = m['action']; t = a.get('type')
    if t == 'customize' and v['page'] == 'maintenance': v['page'] = 'components'; push()
    elif t == 'toggle-component' and v['page'] == 'components': v['components'][a['id']] = not v['components'][a['id']]; push()
    elif t == 'back' and v['page'] == 'components': v['page'] = 'maintenance'; push()
    elif t == 'install' and v['page'] in ('maintenance', 'components', 'welcome'): threading.Thread(target=run_install, daemon=True).start()
    elif t == 'open-panel' and v['page'] == 'done':
        v['opening'] = True; push()
        def stop():
            time.sleep(1.8); v['opening'] = False; push()
        threading.Thread(target=stop, daemon=True).start()
    elif t == 'exit' and not v['busy']:
        p.stdin.close(); break
try: p.wait(timeout=15)
except subprocess.TimeoutExpired: p.kill()
log.write('ui exit %s\n' % p.returncode); log.close()
time.sleep(1.2)
xvfb.terminate()
print('ui exit', p.returncode)
