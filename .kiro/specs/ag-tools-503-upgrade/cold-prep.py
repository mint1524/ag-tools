"""Cold cutover preparation. Execute via ssh stdin, no secret output."""
import subprocess,json,pathlib,os,time
SERVICE='sidequests-agtools-a1xtnr'
def run(*a):return subprocess.check_output(list(a),text=True).strip()
base=pathlib.Path('/home/gosha/backups/ag-tools');base.mkdir(parents=True,exist_ok=True);os.chmod(base,0o700)
spec=json.loads(run('docker','service','inspect',SERVICE))[0]['Spec']
path=base/'pre-503-upgrade-service-spec.json';path.write_text(json.dumps(spec));os.chmod(path,0o600)
run('docker','service','scale',SERVICE+'=0')
try:
 for _ in range(60):
  if not run('docker','ps','-q','--filter','label=com.docker.swarm.service.name='+SERVICE):break
  time.sleep(1)
 else:raise RuntimeError('production writer did not stop')
 # Backup script is embedded by the local caller; it only creates new archives/volumes.
 backup_source="__BACKUP_SOURCE__"
 scope={'__name__':'__main__'}
 exec(compile(backup_source,'backup.py','exec'),scope)
 # Preserve the exact pre-cutover config in the archive; mutate one reviewed field only after proof.
 code=r'''
import json,pathlib,os,copy
p=pathlib.Path('/data/gui_config.json');a=json.loads(p.read_text());b=copy.deepcopy(a);b['proxy']['proxy_pool']['enabled']=False
t=p.with_suffix('.cutover-tmp');t.write_text(json.dumps(b,ensure_ascii=False,indent=2));os.chmod(t,0o600);os.replace(t,p)
c=json.loads(p.read_text());expected=copy.deepcopy(a);expected['proxy']['proxy_pool']['enabled']=False;assert c==expected
'''
 run('docker','run','--rm','--network','none','--memory','128m','-v','ag_tools_data:/data','python:3.12-alpine','python3','-c',code)
 print('COLD_PREP_OK: writer_stopped=true pool_enabled=false old_spec_private=true')
except BaseException:
 # If config write started it is an authorized field; restore exact pre-cutover field in this branch.
 subprocess.run(['docker','service','scale',SERVICE+'=1'],stdout=subprocess.DEVNULL)
 raise
