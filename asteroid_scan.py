"""On-demand scan OCR; never runs in the continuous navigation loop."""
import re,math
from pathlib import Path
from mining_bookmarks import ORE_NAMES,clean_mining

def parse_scan(lines):
    text='\n'.join(lines);data={};rows=[]
    for key,label in [('mass','MASS'),('resistance','RESISTANCE'),('instability','INSTABILITY')]:
        m=re.search(label.replace('INSTABILITY','[IJ]NSTABILITY')+r'\s*:?[^\d\n]{0,4}(\d+(?:\.\d+)?)',text,re.I)
        data[key]=float(m[1]) if m else None
    m=re.search(r'(\d+(?:\.\d+)?)\s*SCU',text,re.I);data['scu']=float(m[1]) if m else None
    for line in lines:
        inert=re.search(r'(\d+(?:\.\d+)?)\s*%\s*INERT(?:\s|MATER)',line,re.I)
        if inert:
            # Inert material has no ore quality; use zero even if its HUD zero is missed.
            rows.append({'ore':'Inert Materials','percent':float(inert[1]),'quality':0.0})
            continue
        m=re.search(r'(\d+(?:\.\d+)?)\s*%\s*(.*?)\s*(\d+(?:\.\d+)?)\s*$',line)
        if not m:continue
        name=re.sub(r'[^a-z]','',re.sub(r'\(?ORE\)?','',m[2],flags=re.I).lower())
        ore=next((o for o in ORE_NAMES if o.lower()==name),None)
        if ore:rows.append({'ore':ore,'percent':float(m[1]),'quality':float(m[3])})
    data['composition']=rows
    return clean_mining(data)

def read_scan(image,reader):
    import cv2,numpy as np
    from rapidocr_onnxruntime import utils
    from rapidocr_onnxruntime.ch_ppocr_v3_det import text_detect
    import rapidocr_onnxruntime as package
    base=Path(package.__file__).parent
    config=utils.read_yaml(str(base/'config.yaml'))['Det'];config['model_path']=str(base/config['model_path'])
    original=text_detect.OrtInferSession;text_detect.OrtInferSession=type(reader.engine.session)
    try:detector=text_detect.TextDetector(config)
    finally:text_detect.OrtInferSession=original
    # Bound detector work for large displays; enlarge the resulting text crops separately.
    scale=min(2,2048/image.width)
    pixels=np.asarray(image.convert('RGB'));small=cv2.resize(pixels,None,fx=scale,fy=scale)
    boxes,_=detector(small)
    raw=[]
    for box in sorted(boxes,key=lambda b:b[:,1].mean()):
        x,y=box.min(axis=0)/scale;xx,yy=box.max(axis=0)/scale
        raw.append(list(map(float,(x,y,xx,yy))))
    def recognize(groups):
        evidence=[]
        for b in groups:
            x,y,xx,yy=b;variants=[]
            for left,top,right,bottom in [(2,1,3,2),(1,4,4,0),(0,3,3,0)]:
                crop=pixels[max(0,int(y)-top):min(image.height,int(yy)+bottom),max(0,int(x)-left):min(image.width,int(xx)+right)]
                if crop.size:
                    crop=cv2.resize(crop,None,fx=4,fy=4,interpolation=cv2.INTER_LINEAR)
                    variants.append(reader.engine([crop])[0][0])
            if not variants:continue
            t,score=max(variants,key=lambda r:r[1])
            evidence.append({'box':b,'text':t,'confidence':float(score),'variants':[v[0] for v in variants]})
        return evidence
    # Find the heading before merging any table rows. Its location is unrestricted.
    evidence=recognize(raw)
    def is_heading(e):
        return any(re.search(r'SCANRESULT[S5]?',re.sub(r'[^A-Z0-9]','',t.upper())) for t in e['variants'])
    headings=[e for e in evidence if is_heading(e)]
    # Some detector passes split SCAN and RESULTS into separate boxes.
    if not headings:
        for e in evidence:
            if not any(re.fullmatch(r'SCAN',t.strip(),re.I) for t in e['variants']):continue
            x,y,xx,yy=e['box'];h=yy-y
            for other in evidence:
                ox,oy,oxx,oyy=other['box']
                if xx-h<=ox<=xx+3*h and abs((oy+oyy-y-yy)/2)<h*.6 and any(re.fullmatch(r'RESULT[S5]?',t.strip(),re.I) for t in other['variants']):
                    headings.append({'box':[x,min(y,oy),oxx,max(yy,oyy)],'confidence':min(e['confidence'],other['confidence'])})
    reader.scan_evidence=evidence
    if not headings:raise ValueError('Cannot locate SCAN RESULTS anywhere in the captured game image. Last capture saved for diagnosis.')
    candidates=[]
    for heading in headings:
        hx,hy,hxx,hyy=heading['box'];width=hxx-hx;height=hyy-hy
        panel=[b for b in raw if hx-width*.15<=b[0]<=hx+width*2.1 and hyy<=b[1]<=hyy+height*24 and b[2]<=hx+width*2.1]
        groups=[]
        for x,y,xx,yy in panel:
            cy=(y+yy)/2
            group=next((g for g in groups if abs(cy-(g[1]+g[3])/2)<min(yy-y,g[3]-g[1])*.55),None)
            if group:group[:]=[min(x,group[0]),min(y,group[1]),max(xx,group[2]),max(yy,group[3])]
            else:groups.append([x,y,xx,yy])
        table=recognize(groups)
        lines=[]
        for row in table:
            x,y,xx,yy=row['box']
            fragments=[e for e in evidence if x-.1<=e['box'][0] and e['box'][2]<=xx+.1 and y-.1<=e['box'][1] and e['box'][3]<=yy+.1 and e['confidence']>=.65]
            assembled=' '.join(e['text'] for e in sorted(fragments,key=lambda e:e['box'][0]))
            # Narrow numeric crops avoid label/background interference. Fall back to
            # whole-row recognition when a fragmented ore name cannot be parsed.
            if len(fragments)>1 and parse_scan([assembled])['composition']:
                lines.append(assembled)
            elif len(fragments)>1 and '%' not in assembled and re.search(r'MASS|RESISTANCE|INSTABILITY|SCU',assembled,re.I):
                lines.append(assembled)
            elif row['confidence']>=.65:lines.append(row['text'])
        try:mining=parse_scan(lines)
        except ValueError:continue
        if mining['scu'] is not None and mining['composition']:
            candidates.append((heading['confidence'],mining,table))
    if not candidates:raise ValueError('Located SCAN RESULTS, but could not read its composition. Last capture saved for diagnosis.')
    _,mining,table=max(candidates,key=lambda c:c[0])
    evidence+=table;reader.scan_evidence=evidence
    ranges=[]
    ore_labels=[e for e in evidence if any(re.search(r'\(?ORE\)?',t,re.I) for t in e['variants']) and '%' not in e['text'] and e['box'][0]<image.width*.72]
    for label in ore_labels:
        x,y,xx,yy=label['box'];h=yy-y;cx=(x+xx)/2
        if not (image.width*.15<cx<image.width*.70 and image.height*.25<y<image.height*.78):continue
        # The range sits immediately below the world-space ore label.
        nearby=[e for e in evidence if abs((e['box'][0]+e['box'][2])/2-cx)<max(xx-x,h)*.7 and yy<=e['box'][1]<=yy+2*h and re.fullmatch(r'\s*\d+(?:\.\d+)?\s*(?:km|m)\s*',e['text'],re.I)]
        distance_box=[max(0,x),yy+h*.15,min(image.width,xx),min(image.height,yy+1.6*h)]
        readings=nearby or recognize([distance_box])
        if not readings:continue
        candidate=readings[0]
        for text in [candidate['text']]+candidate['variants']:
            match=re.fullmatch(r'\s*(\d+(?:\.\d+)?)\s*(km|m)\s*',text,re.I)
            if match and candidate['confidence']>=.65:
                distance=float(match[1])*(1000 if match[2].lower()=='km' else 1)
                ranges.append((((cx/image.width-.5)**2+((yy+h)/image.height-.5)**2),distance))
                break
    if not ranges:raise ValueError('Cannot read the selected rock distance. Center the rock and try again.')
    return mining,min(ranges)[1],evidence






