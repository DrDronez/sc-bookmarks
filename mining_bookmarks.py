"""Mining bookmark data. Ore names from https://scminer.rocks/data/mineral-directory (2026-10-18).
Only names are copied; Inert Materials is included for recording the full scan.
"""
import math

ORE_NAMES=sorted(['Quantainium','Stileron','Riccite','Gold','Borase','Lindinium','Taranite',
    'Bexalite','Agricium','Tungsten','Laranite','Titanium','Torite','Ice','Beryl','Hephaestanite',
    'Tin','Aluminium','Copper','Corundum','Iron','Silicon','Quartz','Aslarite','Carinite','Ouratite','Savrilium'])
COMPOSITION_TYPES=ORE_NAMES+['Inert Materials']

def number(value,label,maximum=1e12):
    if value is None or isinstance(value,str) and not value.strip():return None
    if isinstance(value,bool):raise ValueError(label+' must be a number.')
    try:value=float(value)
    except (ValueError,TypeError):raise ValueError(label+' must be a number or blank.')
    if not math.isfinite(value) or not 0<=value<=maximum:raise ValueError(f'{label} must be between 0 and {maximum:g}.')
    return value

def clean_mining(data):
    if not isinstance(data,dict):raise ValueError('Invalid mining data.')
    result={k:number(data.get(k),k.title(),100 if k=='resistance' else 1e12)
            for k in ('mass','resistance','instability','scu')}
    rows=data.get('composition',[])
    if not isinstance(rows,list) or len(rows)>100:raise ValueError('Use at most 100 composition entries.')
    result['composition']=[]
    for row in rows:
        if not isinstance(row,dict):raise ValueError('Invalid composition entry.')
        ore=row.get('ore','')
        if ore not in COMPOSITION_TYPES:raise ValueError('Unknown ore type: '+str(ore))
        result['composition'].append({'ore':ore,'percent':number(row.get('percent'),'Composition %',100),
                                      'quality':number(row.get('quality'),'Quality')})
    return result

def estimated_scu(scu,row):
    if scu is None or row.get('percent') is None or row.get('quality') is None:return None
    return scu*row['percent']/100

def matches(bookmark,ore='',minimum=None):
    if not ore and minimum is None:return True
    if bookmark.get('category')!='Mining':return False
    return any((not ore or row.get('ore')==ore) and
               (minimum is None or row.get('quality') is not None and row['quality']>=minimum)
               for row in bookmark.get('mining',{}).get('composition',[]))

def best_quality(bookmark,ore=''):
    return max((row['quality'] for row in bookmark.get('mining',{}).get('composition',[])
                if (not ore or row.get('ore')==ore) and row.get('quality') is not None),default=-1)

def filter_mining(rows,ore,minimum,sort):
    rows=[b for b in rows if matches(b,ore,minimum)]
    if sort=='Ore quality':return sorted(rows,key=lambda b:best_quality(b,ore),reverse=True)
    if sort=='Ore type':return sorted(rows,key=lambda b:(
        min((r['ore'] for r in b.get('mining',{}).get('composition',[]) if not ore or r['ore']==ore),default='~'),
        -best_quality(b,ore),b['name'].casefold()))
    return rows
