import ast,json,math
from pathlib import Path
root=Path.cwd();scope={'math':math}
for filename,names in [('overlay_settings.py',{'position','placement'}),('display_geometry.py',{'debug_box'})]:
    tree=ast.parse((root/filename).read_text(encoding='utf-8-sig'))
    exec(compile(ast.Module(body=[n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name in names],type_ignores=[]),filename,'exec'),scope)
cases=[]
for w,h in [(1280,720),(1920,1080),(2560,1440),(3440,1440),(3840,2160),(5120,1440),(400,240),(1921,1081)]:
 for left,top in [(0,0),(-1920,-40),(231,117)]:
  for pos in [[0,0],[50,50],[100,100],[23.5,76.5]]:
   rect=[left,top,w,h];cases.append(dict(rect=rect,position=pos,expected=scope['placement'](rect,pos),debug=scope['debug_box'](w,h)))
(root/'rust/tests/overlay_reference.json').write_text(json.dumps(cases,indent=2),encoding='utf-8')
print(len(cases))
