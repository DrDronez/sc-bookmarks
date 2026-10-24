import ast,json,math,random
from pathlib import Path
root=Path.cwd();tree=ast.parse((root/'system_map.py').read_text(encoding='utf-8-sig'));scope={'math':math}
exec(compile(ast.Module(body=[n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name=='project'],type_ignores=[]),'reference','exec'),scope)
r=random.Random(640);rows=[]
for _ in range(120):
 p=[r.uniform(-1e8,1e8) for _ in range(3)];center=[r.uniform(-1e8,1e8) for _ in range(3)];yaw=r.uniform(-math.pi,math.pi);tilt=r.uniform(.001,math.pi/2);scale=10**r.uniform(-7,-3)
 rows.append(dict(point=p,center=center,yaw=yaw,tilt=tilt,scale=scale,expected=scope['project'](p,center,yaw,tilt,scale,0,0)))
(root/'rust/tests/map_reference.json').write_text(json.dumps(rows,indent=2),encoding='utf-8')
