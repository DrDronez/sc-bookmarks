"""Regenerate fixtures from pure reference code without importing its GUI/OCR."""
import ast,csv,json,re
from pathlib import Path
root=Path.cwd()
scope={'re':re,'csv':csv,'DATA':root/'scanning-data','CHARTS':{'Mining — Space':'mining-space.csv','Mining — Planetary Surface':'mining-planetary-surface.csv','Salvage':'salvage.csv'}}
tree=ast.parse((root/'scanning_reference.py').read_text(encoding='utf-8-sig'))
exec(compile(ast.Module(body=[n for n in tree.body if isinstance(n,ast.FunctionDef)],type_ignores=[]),'reference','exec'),scope)
scan=ast.parse((root/'scan_contacts.py').read_text(encoding='utf-8-sig'))
reader=next(n for n in scan.body if isinstance(n,ast.ClassDef) and n.name=='ScanReader')
method=next(n for n in reader.body if isinstance(n,ast.FunctionDef) and n.name=='description')
exec(compile(ast.Module(body=[method],type_ignores=[]),'reference','exec'),scope)
charts=scope['load_charts']()
values=sorted({v for c in charts.values() for row in c['rows'] for v in row if isinstance(v,int)}|{1,999999})
class Reference:pass
obj=Reference();obj.charts=charts
cases=[{'rs':v,'description':scope['description'](obj,v),'matches':[[i for i,row in enumerate(c['rows']) if scope['matching_columns'](name,row,v)] for name,c in charts.items()]} for v in values]
(root/'rust/tests/scanning_reference.json').write_text(json.dumps(cases,indent=2),encoding='utf-8')
print(f'{len(cases)} distinct readings')
