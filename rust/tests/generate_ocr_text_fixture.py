import ast,json,re
from pathlib import Path
root=Path.cwd();tree=ast.parse((root/'unit_coordinates.py').read_text(encoding='utf-8-sig'));scope={'re':re}
exec(compile(ast.Module(body=[n for n in tree.body if isinstance(n,(ast.Assign,ast.FunctionDef))],type_ignores=[]),'reference','exec'),scope)
texts=['Zone: Root Pos: 43195265.2665km 11254403.6632km 10.6941km','1.0000km-2.0000km+3.0000km','1.00m 2.00m -3.00m','1,0000km 2,0000km −3,0000km','1.000km 2.0000km 3.0000km','1km 2km 3km','1.0000km 2.0000km 3.0000km 4.0000km','a1.0000km 2.0000km 3.0000km','1.0000kmx 2.0000km 3.0000km','1.0000KM 2.0000KM 3.0000KM','+1.0000km 0.0000km -0.00m','1.00000km 2.0000km 3.0000km']
for i in range(40):texts.append(f'{i*127.3123:.4f}km {-i*.0193:.4f}km {i*.01:.2f}m')
cases=[]
for text in texts:
 try:expected=scope['parse_units'](text)
 except ValueError:expected=None
 cases.append(dict(text=text,expected=expected))
(root/'rust/tests/ocr_text_reference.json').write_text(json.dumps(cases,indent=2),encoding='utf-8')
