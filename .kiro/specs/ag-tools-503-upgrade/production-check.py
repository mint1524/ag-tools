"""Remote post-deployment readback, credential-safe evidence only."""
import subprocess,json,pathlib,time,urllib.request,urllib.error,urllib.parse,re
SERVICE='sidequests-agtools-a1xtnr';IMAGE='docker.io/lbjlaq/antigravity-manager:v5.0.3@sha256:a10cc6e4580da081b84c86bdd715bd48e7867ad859d8adeecedf266203b70d51'
def run(*a):return subprocess.check_output(list(a),text=True).strip()
s=json.loads(run('docker','service','inspect',SERVICE))[0]
old=json.loads(pathlib.Path('/home/gosha/backups/ag-tools/pre-503-upgrade-service-spec.json').read_text())
now=s['Spec'];oc=old['TaskTemplate']['ContainerSpec'];nc=now['TaskTemplate']['ContainerSpec']
assert nc['Image']==IMAGE
scope_checks={'mounts_preserved':oc.get('Mounts')==nc.get('Mounts'),'env_preserved':sorted(oc.get('Env',[]))==sorted(nc.get('Env',[])),'networks_preserved':old['TaskTemplate'].get('Networks')==now['TaskTemplate'].get('Networks'),'replicas':now['Mode']['Replicated']['Replicas']==1}
assert all(scope_checks.values())
key=dict(x.split('=',1) for x in nc['Env'])['API_KEY']
def req(path,data=None,auth=True):
 h={'Content-Type':'application/json'}
 if auth:h['Authorization']='Bearer '+key
 r=urllib.request.Request(urllib.parse.urljoin('https://ai.okak.club/',path),data=json.dumps(data).encode() if data is not None else None,headers=h)
 try:
  with urllib.request.urlopen(r,timeout=60) as z:return z.status,z.read().decode()
 except urllib.error.HTTPError as e:return e.code,e.read().decode()
health=None
for _ in range(45):
 try:
  st,b=req('/health',auth=False)
  if st==200 and json.loads(b).get('version')=='5.0.3':health=json.loads(b);break
 except (OSError,ValueError):pass
 time.sleep(2)
assert health,'production readiness failed'
ui,html=req('/',auth=False);assert ui==200
assets=re.findall(r'(?:src|href)="([^"]+\.(?:js|css))"',html)
ast={a:req(a,auth=False)[0] for a in assets};assert assets and all(x==200 for x in ast.values())
st,b=req('/v1/models');assert st==200;models=json.loads(b)['data']
anon,_=req('/v1/models',auth=False);assert anon in [401,403]
probes=[]
for model in ['gemini-3.8-flash-high','claude-sonnet-4-6']:
 st,b=req('/v1/chat/completions',{'model':model,'messages':[{'role':'user','content':'Reply exactly OK'}],'max_tokens':16,'stream':False})
 try:j=json.loads(b);err=j.get('error') or {};code=err.get('code');typ=err.get('type')
 except ValueError:code=None;typ=None
 probes.append({'model':model,'http':st,'error_code':code,'error_type':typ,'tos_violation':bool(re.search('TOS_VIOLATION|account_forbidden|violation of Terms',b,re.I)),'network_error':bool(re.search('upstream_network_error',b,re.I)),'generated':st==200})
code=r'''
import pathlib,json,sqlite3,contextlib,copy
s=pathlib.Path('/src');d=pathlib.Path('/dst')
a=json.loads((s/'accounts.json').read_text());b=json.loads((d/'accounts.json').read_text());ids=lambda x:sorted(v['id'] for v in x['accounts']);assert ids(a)==ids(b) and len(ids(b))==1
for aid in ids(a):
 x=json.loads((s/'accounts'/str(aid+'.json')).read_text());y=json.loads((d/'accounts'/str(aid+'.json')).read_text());assert x['token'].get('refresh_token')==y['token'].get('refresh_token')
a=json.loads((s/'gui_config.json').read_text());b=json.loads((d/'gui_config.json').read_text())
for k in ['api_key','admin_password','auth_mode']:assert a['proxy'].get(k)==b['proxy'].get(k),'auth changed'
assert b['proxy']['proxy_pool']['enabled']==False
for k in ['proxies','account_bindings']:assert a['proxy']['proxy_pool'].get(k)==b['proxy']['proxy_pool'].get(k),'proxy entries changed'
cols='id,token,enabled,expires_type,expires_at,max_ips,curfew_start,curfew_end'
def rows(p):
 uri=(p/'user_tokens.db').as_uri()+('?mode=ro&immutable=1' if p==s else '?mode=ro')
 with contextlib.closing(sqlite3.connect(uri,uri=True)) as c:return c.execute('select '+cols+' from user_tokens order by id').fetchall()
assert rows(s)==rows(d)
for f in d.rglob('*.json'):json.loads(f.read_text())
checks={}
for f in d.rglob('*.db'):
 with contextlib.closing(sqlite3.connect(f.as_uri()+'?mode=ro',uri=True,timeout=20)) as c:checks[str(f.relative_to(d))]=c.execute('PRAGMA integrity_check').fetchone()[0]
assert all(x=='ok' for x in checks.values())
print(json.dumps({'account_identity_preserved':True,'indexed_accounts':len(ids(json.loads((d/'accounts.json').read_text()))),'refresh_token_preserved':True,'auth_settings_preserved':True,'pool_enabled':False,'proxy_list_and_bindings_preserved':True,'client_token_policy_preserved':True,'client_tokens':len(rows(d)),'json_valid':True,'sqlite':checks}))
'''
data=json.loads(run('docker','run','--rm','--network','none','--memory','256m','--cpus','0.5','-v','ag_tools_backup_20261113T113808Z:/src:ro','-v','ag_tools_data:/dst','python:3.12-alpine','python3','-c',code))
cid=run('docker','ps','-q','--filter','label=com.docker.swarm.service.name='+SERVICE);assert len(cid.splitlines())==1
c=json.loads(run('docker','inspect',cid))[0]
assert c['State']['Running']
logs=subprocess.run(['docker','logs','--since','10m',cid],text=True,capture_output=True);txt=logs.stdout+logs.stderr
out={'service':SERVICE,'image':nc['Image'],'container_image_id':c['Image'],'running_containers':1,'scope':scope_checks,'health':health,'ui_http':ui,'assets':ast,'models_http':200,'models_count':len(models),'model_ids':[x['id'] for x in models],'anonymous_models_http':anon,'data':data,'generation_probes':probes,'log_flags':{'tos_violation':'TOS_VIOLATION' in txt,'permission_denied':'PERMISSION_DENIED' in txt,'panic':bool(re.search(r'panicked at|thread.*panicked',txt))},'rollback_executed':False}
assert not out['log_flags']['panic']
print(json.dumps(out,indent=2))
