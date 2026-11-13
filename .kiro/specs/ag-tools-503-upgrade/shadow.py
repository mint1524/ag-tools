"""Remote bounded isolated image acceptance. Run via ssh python3 stdin."""
import subprocess,json,time,urllib.request,urllib.error,urllib.parse,re
IMAGE='docker.io/lbjlaq/antigravity-manager:v5.0.3@sha256:a10cc6e4580da081b84c86bdd715bd48e7867ad859d8adeecedf266203b70d51'
SNAPSHOT='ag_tools_backup_20261113T112619Z'
VOLUME='ag_tools_shadow_20261113T112619Z'
NAME='ag-tools-shadow-20261113'
SERVICE='sidequests-agtools-a1xtnr'
def run(*args):return subprocess.check_output(list(args),text=True).strip()
s=json.loads(run('docker','service','inspect',SERVICE))[0]
old_spec=s['Spec']
env=s['Spec']['TaskTemplate']['ContainerSpec'].get('Env',[])
run('docker','volume','create',VOLUME)
if subprocess.run(['docker','inspect',NAME],capture_output=True).returncode!=0:
 run('docker','run','--rm','--network','none','--memory','256m','--cpus','0.5','-v',SNAPSHOT+':/src:ro','-v',VOLUME+':/dst','python:3.12-alpine','python3','-c',"import shutil;shutil.copytree('/src','/dst',dirs_exist_ok=True)")
args=['docker','run','-d','--name',NAME,'--restart','no','--memory','512m','--cpus','0.5','--pids-limit','256','-v',VOLUME+':/root/.antigravity_tools','-p','127.0.0.1:18046:8045']
for e in env:args+=['-e',e]
args+=[IMAGE]
existing=subprocess.run(['docker','inspect',NAME],capture_output=True,text=True)
if existing.returncode==0:
 assert json.loads(existing.stdout)[0]['State']['Status']=='exited'
 run('docker','start',NAME)
else:
 run(*args)
d=json.loads(run('docker','inspect',NAME))[0]
assert d['HostConfig']['RestartPolicy']['Name']=='no'
assert d['HostConfig']['PortBindings']['8045/tcp']==[{'HostIp':'127.0.0.1','HostPort':'18046'}]
assert [m['Name'] for m in d['Mounts'] if m['Type']=='volume']==[VOLUME]
assert not any('traefik' in k.lower() or 'dokploy' in k.lower() for k in d['Config'].get('Labels',{}))
key=dict(x.split('=',1) for x in env)['API_KEY']
def req(path,data=None,auth=True):
 headers={'Content-Type':'application/json'}
 if auth:headers['Authorization']='Bearer '+key
 r=urllib.request.Request(urllib.parse.urljoin('http://127.0.0.1:18046/',path),data=json.dumps(data).encode() if data is not None else None,headers=headers)
 try:
  with urllib.request.urlopen(r,timeout=50) as z:return z.status,z.read().decode()
 except urllib.error.HTTPError as e:return e.code,e.read().decode()
def ready():
 for _ in range(30):
  try:
   code,b=req('/health',auth=False)
   if code==200:
    h=json.loads(b);assert h['version']=='5.0.3';return h
  except (urllib.error.URLError,TimeoutError,ConnectionResetError,ConnectionRefusedError):pass
  time.sleep(1)
 raise RuntimeError('shadow readiness failed')
check_code=r'''
import pathlib,json,sqlite3,hashlib,contextlib
s=pathlib.Path('/src');d=pathlib.Path('/dst')
a=json.loads((s/'accounts.json').read_text());b=json.loads((d/'accounts.json').read_text())
ids=lambda x:sorted(v['id'] for v in x['accounts'])
assert ids(a)==ids(b) and len(ids(b))==1
for aid in ids(a):
 x=json.loads((s/'accounts'/str(aid+'.json')).read_text());y=json.loads((d/'accounts'/str(aid+'.json')).read_text())
 assert x['token'].get('refresh_token')==y['token'].get('refresh_token')
x=json.loads((s/'gui_config.json').read_text());y=json.loads((d/'gui_config.json').read_text())
for k in ['api_key','admin_password','auth_mode']:
 assert x['proxy'].get(k)==y['proxy'].get(k), 'auth setting changed: '+k
checks={}
for f in d.rglob('*.json'):json.loads(f.read_text())
for f in d.rglob('*.db'):
 with contextlib.closing(sqlite3.connect(str(f))) as c:checks[str(f.relative_to(d))]=c.execute('PRAGMA integrity_check').fetchone()[0]
assert all(x=='ok' for x in checks.values())
# Client credential identity and admission fields must survive schema migration.
columns='id,token,enabled,expires_type,expires_at,max_ips,curfew_start,curfew_end'
def tokens(p):
 with contextlib.closing(sqlite3.connect((p/'user_tokens.db').as_uri()+('?mode=ro&immutable=1' if p==s else ''),uri=True)) as c:return c.execute('select '+columns+' from user_tokens order by id').fetchall()
assert tokens(s)==tokens(d),'client credential policy changed'
print(json.dumps({'account_identity_preserved':True,'indexed_accounts':len(ids(b)),'refresh_token_preserved':True,'auth_config_preserved':True,'client_token_policy_preserved':True,'client_tokens':len(tokens(d)),'json_valid':True,'sqlite':checks}))
'''
def check():return json.loads(run('docker','run','--rm','--network','none','--memory','256m','--cpus','0.5','-v',SNAPSHOT+':/src:ro','-v',VOLUME+':/dst','python:3.12-alpine','python3','-c',check_code))
try:
 first=ready();run('docker','stop','--time','30',NAME);first_data=check()
 run('docker','start',NAME);second=ready()
 status,body=req('/v1/models');assert status==200
 models=json.loads(body)['data'];anonymous,_=req('/v1/models',auth=False);assert anonymous in [401,403]
 ui,html=req('/',auth=False);assert ui==200
 assets=re.findall(r'(?:src|href)="([^"]+\.(?:js|css))"',html)
 asset_statuses={a:req(a,auth=False)[0] for a in assets};assert assets and all(x==200 for x in asset_statuses.values())
 # One real bounded request; no fake success and no attempt to bypass ban.
 status,body=req('/v1/chat/completions',{'model':'gemini-3.8-flash-high','messages':[{'role':'user','content':'Reply exactly OK'}],'max_tokens':16,'stream':False})
 error={'http':status,'tos_violation':bool(re.search('TOS_VIOLATION|account_forbidden|violation of Terms',body,re.I)),'pool_error':bool(re.search('no.available|no.quota|accounts.*(exhaust|limit|forbidden)|account_forbidden',body,re.I)),'generated':False}
 if status==200:
  j=json.loads(body);error['generated']=bool(j.get('choices'))
 assert status!=200 or error['generated'],'false200 generation'
 run('docker','stop','--time','30',NAME);second_data=check()
 assert json.loads(run('docker','service','inspect',SERVICE))[0]['Spec']==old_spec,'production spec changed during shadow'
 out={'name':NAME,'volume':VOLUME,'image':IMAGE,'isolation':{'loopback_only':True,'production_volume_mounted':False,'traefik_labels':False,'restart':'no','memory_limit':536870912,'cpu_limit':0.5},'first_health':first,'first_data':first_data,'second_health':second,'second_data':second_data,'models_http':200,'models_count':len(models),'model_ids':[m['id'] for m in models],'anonymous_models_http':anonymous,'ui_http':ui,'assets':asset_statuses,'generation':error,'final_state':'exited','production_spec_unchanged':True}
 print(json.dumps(out,indent=2))
finally:
 subprocess.run(['docker','stop','--time','30',NAME],stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)
