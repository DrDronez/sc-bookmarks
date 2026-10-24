import ast,json
from pathlib import Path
root=Path.cwd();scope={}
for filename,names in [('bookmark_links.py',{'organize'}),('mining_bookmarks.py',{'matches','best_quality','filter_mining'})]:
    tree=ast.parse((root/filename).read_text(encoding='utf-8-sig'))
    exec(compile(ast.Module(body=[n for n in tree.body if isinstance(n,ast.FunctionDef) and n.name in names],type_ignores=[]),filename,'exec'),scope)
bookmarks=[]
for i,(name,cat,system,ore,q) in enumerate([('Alpha','Mining','Stanton','Iron',664),('Beta','Mining','Pyro','Gold',800),('Gamma','Personal','Stanton','Iron',999),('Delta','Mining','Stanton','Iron',None),('Echo','Mining','Nyx','Iron',500),('Foxtrot','Salvage','Stanton','Gold',0),('Hotel','Custom','Stanton','Iron',12)]):
    bookmarks.append(dict(name=name,category=cat,system=system,xyz_km=[i,0,0],utc=f'2026-10-{20-i:02d}T00:00:00Z',mining={'composition':[{'ore':ore,'quality':q},{'ore':'Inert Materials','quality':0}]}))
categories=['Mining','Salvage','Personal','Corp','Custom'];cases=[]
for sort in ['Category','Name','System','Newest','Ore type','Ore quality']:
 for category in ['All categories','Mining']:
  for system in ['', 'Stanton']:
   for ore,minimum in [('',None),('Iron',500),('',0)]:
    rows=scope['organize'](bookmarks,category,system,True,sort,categories)
    rows=scope['filter_mining'](rows,ore,minimum,sort)
    cases.append(dict(sort=sort,category='' if category=='All categories' else category,system=system,ore=ore,minimum=minimum,indices=[bookmarks.index(b) for b in rows]))
(root/'rust/tests/library_reference.json').write_text(json.dumps(dict(bookmarks=bookmarks,categories=categories,cases=cases),indent=2),encoding='utf-8')
print(len(cases))
