import ast,json,math,random
from pathlib import Path
root=Path.cwd()
tree=ast.parse((root/'SCBookmarks.py').read_text(encoding='utf-8-sig'))
names={'sub','dot','norm','approach','camera_basis'}
scope={'math':math}
exec(compile(ast.Module(body=[n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name in names],type_ignores=[]),'reference','exec'),scope)
rng=random.Random(301)
rows=[]
for _ in range(100):
    p=[rng.uniform(-1e7,1e7) for i in range(3)]
    target=[rng.uniform(-1e7,1e7) for i in range(3)]
    velocity=[rng.uniform(-100,100) for i in range(3)]
    angles=[rng.uniform(-180,180) for i in range(3)]
    d=scope['sub'](target,p);f,r,u=scope['camera_basis'](angles);dot=scope['dot']
    steering=[math.degrees(math.atan2(dot(d,r),dot(d,f))),math.degrees(math.atan2(dot(d,u),math.hypot(dot(d,r),dot(d,f))))]
    rows.append(dict(position=p,target=target,velocity=velocity,angles=angles,approach=scope['approach'](p,target,velocity),steering=steering))
(root/'rust/tests/guidance_reference.json').write_text(json.dumps(rows,indent=2),encoding='utf-8')
