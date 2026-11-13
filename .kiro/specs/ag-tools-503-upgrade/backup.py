"""Run on nlvmv2 via ssh python3 - < backup.py. No credentials emitted."""
import subprocess, json, datetime, os, pathlib, hashlib

def run(*args):
    return subprocess.check_output(list(args), text=True).strip()

tag = datetime.datetime.now(datetime.timezone.utc).strftime('%Y%m%dT%H%M%SZ')
base = pathlib.Path('/home/gosha/backups/ag-tools')
base.mkdir(parents=True, exist_ok=True)
os.chmod(base, 0o700)
snapshot = 'ag_tools_backup_' + tag
restore = 'ag_tools_restore_' + tag
archive = 'ag_tools_data-' + tag + '.tar.gz'
helper = 'python:3.12-alpine'
run('docker', 'volume', 'create', snapshot)
code = r'''
import os,json,pathlib,sqlite3,shutil,hashlib,tarfile,contextlib
s=pathlib.Path('/src'); d=pathlib.Path('/dst')
def hashes(p):
 return {str(f.relative_to(p)):hashlib.sha256(f.read_bytes()).hexdigest() for f in p.rglob('*.json') if f.is_file()}
before=hashes(s)
for f in s.rglob('*'):
 if f.is_symlink(): raise RuntimeError('source symlink refused')
 if not f.is_file() or f.name.endswith(('-wal','-shm','-journal')): continue
 t=d/f.relative_to(s);t.parent.mkdir(parents=True,exist_ok=True)
 if f.suffix=='.db':
  with contextlib.closing(sqlite3.connect(str(f),timeout=30)) as src, contextlib.closing(sqlite3.connect(str(t))) as dst:
   src.backup(dst)
 else: shutil.copy2(f,t)
assert before==hashes(s), 'source JSON changed during backup; retry with new snapshot'
assert before==hashes(d)
json_count=0
for f in d.rglob('*.json'):
 json.loads(f.read_text());json_count+=1
results={}
for f in d.rglob('*.db'):
 with contextlib.closing(sqlite3.connect(f.as_uri()+'?mode=ro&immutable=1',uri=True)) as c:results[str(f.relative_to(d))]=c.execute('PRAGMA integrity_check').fetchone()[0]
assert all(v=='ok' for v in results.values()),results
idx=json.loads((d/'accounts.json').read_text())
assert len(idx['accounts'])==1
out={'source_json_stable':True,'snapshot_json_equal':True,'json_count':json_count,'indexed_accounts':len(idx['accounts']),'sqlite':results}
with tarfile.open('/out/'+os.environ['ARCHIVE'],'w:gz') as t:t.add(d,arcname='.')
os.chmod('/out/'+os.environ['ARCHIVE'],0o600)
print(json.dumps(out))
'''
res = run('docker','run','--rm','--network','none','--memory','256m','--cpus','0.5','-v','ag_tools_data:/src','-v',snapshot+':/dst','-v',str(base)+':/out','-e','ARCHIVE='+archive,helper,'python3','-c',code)
# uid/gid are resolved from the real target user, never assumed.
run('docker','run','--rm','--network','none','-v',str(base)+':/out','-e','ARCHIVE='+archive,'-e','OWNER='+str(os.getuid())+':'+str(os.getgid()),helper,'python3','-c',"import os;u,g=map(int,os.environ['OWNER'].split(':'));os.chown('/out/'+os.environ['ARCHIVE'],u,g)")
run('docker','volume','create',restore)
check = r'''
import tarfile,pathlib,json,sqlite3,hashlib,os,contextlib
p=pathlib.Path('/restore');s=pathlib.Path('/snapshot')
with tarfile.open('/out/'+os.environ['ARCHIVE'],'r:gz') as t:t.extractall(p,filter='data')
a={str(f.relative_to(s)):hashlib.sha256(f.read_bytes()).hexdigest() for f in s.rglob('*') if f.is_file()}
b={str(f.relative_to(p)):hashlib.sha256(f.read_bytes()).hexdigest() for f in p.rglob('*') if f.is_file()}
assert a==b,'restore file bytes mismatch'
for f in p.rglob('*.json'):json.loads(f.read_text())
checks={}
for f in p.rglob('*.db'):
 with contextlib.closing(sqlite3.connect(f.as_uri()+'?mode=ro&immutable=1',uri=True)) as c:checks[str(f.relative_to(p))]=c.execute('PRAGMA integrity_check').fetchone()[0]
assert all(v=='ok' for v in checks.values())
print(json.dumps({'all_restored_file_hashes_match':True,'json_valid':True,'sqlite':checks}))
'''
proof=json.loads(run('docker','run','--rm','--network','none','--memory','256m','--cpus','0.5','-v',snapshot+':/snapshot:ro','-v',restore+':/restore','-v',str(base)+':/out:ro','-e','ARCHIVE='+archive,helper,'python3','-c',check))
p=base/archive
out={'tag':tag,'snapshot':snapshot,'restore':restore,'archive':str(p),'sha256':hashlib.sha256(p.read_bytes()).hexdigest(),'mode':oct(p.stat().st_mode&0o777),'bytes':p.stat().st_size,'backup':json.loads(res),'restore_proof':proof}
print(json.dumps(out,indent=2))
