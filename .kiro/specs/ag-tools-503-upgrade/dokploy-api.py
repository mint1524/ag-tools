"""Local Dokploy maintenance API, outputs only an explicit safe allowlist."""
import argparse,json,urllib.request,yaml
p=argparse.ArgumentParser();p.add_argument('action',choices=['read','set-image','deploy']);p.add_argument('--image');args=p.parse_args()
c=yaml.safe_load(open('/home/mint/.hermes/profiles/work/config.yaml'))
e=c['mcp_servers']['dokploy-mcp']['env']
base=e['DOKPLOY_URL'].rstrip('/')
appid='sJtK_KfVh0AqvO_PIRo_Q'
def call(route,body=None):
 url=base+'/api/'+route
 headers={'x-api-key':e['DOKPLOY_API_KEY'],'Content-Type':'application/json'}
 r=urllib.request.Request(url,headers=headers,data=json.dumps(body).encode() if body is not None else None)
 with urllib.request.urlopen(r,timeout=45) as z:
  b=z.read().decode()
  try:return z.status,json.loads(b)
  except json.JSONDecodeError:return z.status,{'response_type':'non-json'}
if args.action=='read':
 code,d=call('application.one?applicationId='+appid)
 print(json.dumps({'http':code,**{k:d.get(k) for k in ['applicationId','appName','dockerImage','applicationStatus','sourceType']}}))
elif args.action=='set-image':
 assert args.image and args.image.startswith(('docker.io/lbjlaq/antigravity-manager:v5.0.3@sha256:','ghcr.io/mint1524/ag-tools:sha-0066e12'))
 code,_=call('application.saveDockerProvider',{'applicationId':appid,'dockerImage':args.image,'username':None,'password':None,'registryUrl':None})
 print(json.dumps({'http':code,'operation':'saveDockerProvider'}))
else:
 code,_=call('application.deploy',{'applicationId':appid,'title':'AG Tools 5.0.3 maintenance','description':'Owner-approved pinned upstream update; backup/shadow checked. Google account ban remains external.'})
 print(json.dumps({'http':code,'operation':'deploy'}))
