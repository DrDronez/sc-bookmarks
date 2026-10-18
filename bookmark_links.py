"""Portable, versioned bookmark links. Coordinates are always Root-frame kilometres."""
import math,json
from urllib.parse import urlsplit, parse_qs, urlencode

def validate(bookmark):
    result=dict(bookmark)
    name=str(result.get('name','')).strip()
    system=str(result.get('system','')).strip()
    if not name or len(name)>200:raise ValueError('Name must contain 1–200 characters.')
    if not system or len(system)>80:raise ValueError('Enter a solar system (up to 80 characters).')
    xyz=result.get('xyz_km',[])
    if len(xyz)!=3:raise ValueError('Enter X, Y and Z coordinates in kilometres.')
    try:xyz=[float(v) for v in xyz]
    except (TypeError,ValueError):raise ValueError('Coordinates must be numbers.')
    if not all(math.isfinite(v) and abs(v)<=1e12 for v in xyz):raise ValueError('Coordinates must be finite and within ±1 trillion km.')
    shard=result.get('shard_id') or None
    if shard is not None and (len(shard)>128 or any(ord(c)<32 for c in shard)):
        raise ValueError('Invalid shard ID.')
    result.update(name=name,system=system,xyz_km=xyz,shard_id=shard,frame='root',units='km')
    category=str(result.get('category','Personal')).strip()
    if not category or len(category)>80:raise ValueError('Category must contain 1–80 characters.')
    result['category']=category
    if category.casefold()=='mining':result['category']='Mining'
    notes=result.get('notes','')
    if not isinstance(notes,str) or len(notes)>10000:raise ValueError('Notes must be text, up to 10,000 characters.')
    result['notes']=notes
    if 'mining' in result:
        from mining_bookmarks import clean_mining
        result['mining']=clean_mining(result['mining'])
    return result

def encode(bookmark):
    b=validate(bookmark)
    fields={'v':'1','x':repr(b['xyz_km'][0]),'y':repr(b['xyz_km'][1]),'z':repr(b['xyz_km'][2]),
            'units':'km','frame':'root','name':b['name'],'system':b['system']}
    if b.get('shard_id'):fields['shard']=b['shard_id']
    if b.get('utc'):fields['created']=b['utc']
    fields['category']=b['category']
    if b.get('notes') or 'mining' in b:
        fields['v']='2'
        if b.get('notes'):fields['notes']=b['notes']
        if 'mining' in b:fields['mining']=json.dumps(b['mining'],ensure_ascii=False,separators=(',',':'))
    return 'scbm://bookmark?'+urlencode(fields)

def decode(link):
    if len(link)>200000:raise ValueError('Bookmark link is too long.')
    u=urlsplit(link.strip())
    if u.scheme.lower()!='scbm' or u.netloc!='bookmark' or u.path not in ('','/') or u.fragment:
        raise ValueError('Expected an scbm://bookmark link.')
    q=parse_qs(u.query,keep_blank_values=True,max_num_fields=20)
    if any(len(v)!=1 for v in q.values()):raise ValueError('Duplicate fields in bookmark link.')
    q={k:v[0] for k,v in q.items()}
    if q.get('v') not in ('1','2'):raise ValueError('Unsupported bookmark link version.')
    if q.get('units')!='km' or q.get('frame')!='root':raise ValueError('Only Root coordinates in kilometres are supported.')
    try:
        b={'name':q['name'],'system':q['system'],'xyz_km':[q['x'],q['y'],q['z']],
           'shard_id':q.get('shard'),'category':q.get('category','Personal'),'kind':'imported bookmark'}
    except KeyError as e:raise ValueError('Missing bookmark field: '+str(e))
    if q.get('created'):
        from datetime import datetime
        try:datetime.fromisoformat(q['created'].replace('Z','+00:00'))
        except ValueError:raise ValueError('Invalid creation date.')
        b['utc']=q['created']
    b['notes']=q.get('notes','')
    if 'mining' in q:
        try:b['mining']=json.loads(q['mining'])
        except (ValueError,RecursionError):raise ValueError('Invalid mining data in bookmark link.')
    return validate(b)

def decode_many(text):
    if len(text)>4000000:raise ValueError('Paste fewer bookmarks at a time (maximum 4 MB).')
    lines=[line.strip() for line in text.splitlines() if line.strip()]
    if not lines or len(lines)>500:raise ValueError('Paste between 1 and 500 bookmark links, one per line.')
    return [decode(line) for line in lines]

DEFAULT_CATEGORIES=['Mining','Salvage','Personal','Corp']

def organize(bookmarks,category,system,hide_other,sort,categories):
    rows=[b for b in bookmarks if (category=='All categories' or b.get('category','Personal')==category)
          and (not hide_other or not system.strip() or b.get('system','').casefold()==system.strip().casefold())]
    rank={name:i for i,name in enumerate(categories)}
    if sort=='Name':key=lambda b:b['name'].casefold()
    elif sort=='System':key=lambda b:(b.get('system','').casefold(),b['name'].casefold())
    elif sort=='Newest':return sorted(rows,key=lambda b:b.get('utc',''),reverse=True)
    else:key=lambda b:(rank.get(b.get('category','Personal'),len(rank)),b['name'].casefold())
    return sorted(rows,key=key)
