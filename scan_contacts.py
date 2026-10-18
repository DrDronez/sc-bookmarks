"""Occasional, read-only recognition of formatted RS contact badges."""
import re,time,copy
from pathlib import Path
from scanning_reference import load_charts,matching_columns

class ScanReader:
 def __init__(self):
  from unit_coordinates import UnitReader
  from rapidocr_onnxruntime import utils
  from rapidocr_onnxruntime.ch_ppocr_v3_det import text_detect
  import rapidocr_onnxruntime as package
  self.reader=UnitReader();self.charts=load_charts()
  base=Path(package.__file__).parent;cfg=utils.read_yaml(str(base/'config.yaml'))['Det'];cfg['model_path']=str(base/cfg['model_path'])
  original=text_detect.OrtInferSession;text_detect.OrtInferSession=type(self.reader.engine.session)
  try:self.detector=text_detect.TextDetector(cfg)
  finally:text_detect.OrtInferSession=original
 def description(self,value):
  groups={'Surface':[],'Space':[]}
  for chart,data in self.charts.items():
   environment='Surface' if 'Surface' in chart else 'Space'
   for row in data['rows']:
    for count in matching_columns(chart,row,value):
     if chart=='Salvage':
      texts=[re.sub(r'(\d+) scrap piece(s?)',r'\1 Salvage Scrap Piece\2',part.strip(),flags=re.I) for part in row[1].split(' or ')]
     elif row[0] in ('ROC mining','Hand mining'):
      texts=[f'{count} '+row[0].replace(' mining','')+' '+('Deposit' if count==1 else 'Deposits')]
     else:texts=[f'{count} {row[0]} '+('Rock' if count==1 else 'Rocks')]
     for text in texts:
      if text.casefold() not in [v.casefold() for v in groups[environment]]:groups[environment].append(text)
  return f'Scan Results for {value:,} Indicate:\n'+'\n'.join(f'{name:7}: '+(' / '.join(values) if values else 'No exact reference match') for name,values in groups.items())
 def read(self,image):
  now=time.monotonic()
  if getattr(self,'frame_size',None)!=image.size:
   self.frame_size=image.size;self.regions=[];self.badges=[];self.next_search=0
  # Check only the small area around previously located badges between searches.
  if self.regions and now<self.next_search:
   self.mode='badge';found=[]
   for index,(left,top,right,bottom) in enumerate(self.regions):
    quick=self.read_number(image,self.badges[index])
    if quick is not None:
     found.append(quick);continue
    crop=image.crop((left,top,right,bottom))
    for record in self.read_region(crop):
     record['x']=(left+record['x']*crop.width)/image.width
     record['y']=(top+record['y']*crop.height)/image.height
     bx,by,bxx,byy=record['bounds']
     record['bounds']=(bx+left,by+top,bxx+left,byy+top)
     self.badges[index]=dict(record)
     found.append(record)
   return found
  self.mode='search'
  found=self.read_region(image)
  self.regions=[];self.badges=copy.deepcopy(found)
  for record in found:
   x,y,xx,yy=record['bounds'];h=yy-y
   self.regions.append((max(0,int(x-3*h)),max(0,int(y-2*h)),min(image.width,int(xx+3*h)),min(image.height,int(yy+2*h))))
  # A known badge gets cheap checks every second; occasional full searches
  # relocate moved badges and discover additional contacts.
  self.next_search=time.monotonic()+(10 if found else 5)
  return found
 def read_number(self,image,record):
  import cv2,numpy as np
  x,y,xx,yy=record['bounds'];h=yy-y
  box=(max(0,int(x)-1),int(y),min(image.width,int(xx+h*.35)),int(yy))
  crop=np.asarray(image.crop(box).convert('RGB'))
  if not crop.size:return None
  readings=[]
  for method in (cv2.INTER_CUBIC,cv2.INTER_LINEAR):
   enlarged=cv2.resize(crop,None,fx=4,fy=4,interpolation=method)
   if method==cv2.INTER_LINEAR:enlarged=cv2.cvtColor(cv2.cvtColor(enlarged,cv2.COLOR_RGB2GRAY),cv2.COLOR_GRAY2RGB)
   text,score=self.reader.engine([enlarged])[0][0]
   match=re.fullmatch(r'\s*(\d{1,3},\d{3})\s*',text)
   if match and score>=.65:readings.append(int(match[1].replace(',','')))
  if len(readings)!=2 or readings[0]!=readings[1]:return None
  # Verify the nearby icon is still present, rather than refreshing old data.
  icon=np.asarray(image.crop((max(0,int(x-h*1.8)),max(0,int(y-h*.4)),int(x),min(image.height,int(yy+h*.4)))).convert('RGB'))
  if not icon.size:return None
  hsv=cv2.cvtColor(icon,cv2.COLOR_RGB2HSV)
  if ((hsv[:,:,0]>10)&(hsv[:,:,0]<105)&(hsv[:,:,1]>100)&(hsv[:,:,2]>140)).sum()<max(5,h*h*.035):return None
  result=dict(record);result['rs']=readings[0];result['text']=self.description(readings[0])
  return result
 def read_region(self,image):
  import cv2,numpy as np
  pixels=np.asarray(image.convert('RGB'));scale=min(1,1920/image.width)
  boxes,_=self.detector(cv2.resize(pixels,None,fx=scale,fy=scale))
  self.evidence=[];self.vote_evidence=[];found=[]
  if boxes is None:return found
  for box in boxes:
   x,y=box.min(axis=0)/scale;xx,yy=box.max(axis=0)/scale;h=yy-y;w=xx-x
   # RS badges are short standalone text, unlike debug rows and chat.
   if not 5<=h<=max(60,image.height*.05) or not 1.3<=w/h<=7:continue
   bounds=(max(0,int(x)-2),max(0,int(y)-2),min(image.width,int(xx)+2),min(image.height,int(yy)+2))
   crop=np.asarray(image.crop(bounds).convert('RGB'));factor=min(4,48/crop.shape[0])
   text,confidence=self.reader.engine([cv2.resize(crop,None,fx=factor,fy=factor,interpolation=cv2.INTER_CUBIC)])[0][0]
   if re.search(r'\d',text):self.evidence.append((text,float(confidence),bounds))
   if not re.search(r'\d[,.]\d{2}',text):continue
   # The detector may merge the location pin into the text box. Test tightly
   # cropped number-only regions instead of interpreting the pin as a digit.
   votes={};choices={}
   for trim in (0,.5,.75,1.0):
    for inset in (0,.16):
     bx=max(0,int(bounds[0]+trim*(bounds[3]-bounds[1])))
     by=max(0,int(bounds[1]+inset*(bounds[3]-bounds[1])))
     bottom=min(image.height,int(bounds[3]-inset*(bounds[3]-bounds[1])))
     if bounds[2]-bx<12 or bottom-by<6:continue
     sample=pixels[by:bottom,bx:bounds[2]]
     for method in (cv2.INTER_CUBIC,cv2.INTER_LINEAR):
      enlarged=cv2.resize(sample,None,fx=4,fy=4,interpolation=method)
      if method==cv2.INTER_LINEAR:enlarged=cv2.cvtColor(cv2.cvtColor(enlarged,cv2.COLOR_RGB2GRAY),cv2.COLOR_GRAY2RGB)
      reading,score=self.reader.engine([enlarged])[0][0]
      match=re.fullmatch(r'\s*(\d{1,3},\d{3})\s*',reading)
      if not match or score<.65:continue
      value=int(match[1].replace(',',''))
      votes[value]=votes.get(value,0)+1
      if value not in choices or score>choices[value][0]:choices[value]=(score,bx,by,bounds[2],bottom)
   self.vote_evidence.append((text,votes,choices))
   ordered=sorted(votes,key=lambda v:votes[v],reverse=True)
   if not ordered or votes[ordered[0]]<2:continue
   if len(ordered)>1 and votes[ordered[0]]<=votes[ordered[1]]:continue
   value=ordered[0]
   _,x,y,xx,yy=choices[value];h=yy-y
   # A contact badge has a colored location icon immediately to the left.
   icon=pixels[max(0,int(y-h*.4)):min(image.height,int(yy+h*.4)),max(0,int(x-h*1.8)):max(0,int(x))]
   if not icon.size:continue
   hsv=cv2.cvtColor(icon,cv2.COLOR_RGB2HSV)
   colored=((hsv[:,:,0]>10)&(hsv[:,:,0]<105)&(hsv[:,:,1]>100)&(hsv[:,:,2]>140))
   if colored.sum()<max(5,h*h*.035):continue
   found.append({'rs':value,'x':(x+xx)/2/image.width,'y':min(1,(yy+5)/image.height),'text':self.description(value),'bounds':(float(x),float(y),float(xx),float(yy))})
  return sorted(found,key=lambda f:(f['y'],f['x']))[:8]


class ContactOverlay:
 """Small click-through labels; no full-screen transparent surface."""
 def __init__(self,root,factory):self.root=root;self.factory=factory;self.panels=[];self.records=[];self.stamp=0;self.size=None;self.angles=None
 def update(self,stamp,size,records,angles,now=None):
  # An unreadable snapshot must not erase a still-valid description or extend it.
  if not records:return
  self.stamp=time.monotonic() if now is None else now
  self.size=size;self.records=copy.deepcopy(records);self.angles=angles
  # Bounds always use client-image pixels. Rebuild the display anchor here;
  # never let crop-normalized coordinates or later cache mutations move it.
  for record in self.records:
   if 'bounds' in record:
    x,y,xx,yy=record['bounds']
    record['x']=(x+xx)/(2*size[0])
    record['y']=min(1,(yy+5)/size[1])
 def hide(self):
  for panel in self.panels:panel.hide()
 def render(self,geometry,angles,enabled,now):
  if not enabled or not geometry or now-self.stamp>10:
   self.hide();return
  left,top,width,height=geometry
  if self.size and (abs(width-self.size[0])>3 or abs(height-self.size[1])>3):
   self.records=[];self.hide();return
  for i,record in enumerate(self.records):
   if len(self.panels)<=i:self.panels.append(self.factory(self.root))
   panel=self.panels[i];w=min(380,width);h=min(100,height)
   x=left+max(0,min(width-w,round(record['x']*width-w/2)))
   y=top+max(0,min(height-h,round(record['y']*height)))
   showing=not panel.visible
   if showing:panel.window.deiconify();panel.visible=True
   layout=(x,y,w,h,record['text'],self.stamp)
   if not showing and getattr(panel,'scan_layout',None)==layout:continue
   panel.scan_layout=layout
   c=panel.canvas;c.delete('all')
   heading,_,body=record['text'].partition('\n')
   title=c.create_text(w/2,10,text=heading,anchor='n',font=('Segoe UI',-13,'bold'),fill='#f3cb78')
   content=c.create_text(13,35,text=body,anchor='nw',width=w-26,justify='left',font=('Consolas',-13,'bold'),fill='#e2f1e9')
   bounds=c.bbox(content)
   h=min(height,max(88,(bounds[3] if bounds else 70)+13))
   y=top+max(0,min(height-h,round(record['y']*height)))
   panel.user.SetWindowPos(panel.hwnd,-1,x,y,w,h,0x10|0x40|0x4000)
   background=c.create_polygon(1,7,7,1,w-8,1,w-1,8,w-1,h-8,w-8,h-1,8,h-1,1,h-8,fill='#102526',outline='#bc9654',width=1)
   c.tag_lower(background)
   c.create_line(13,29,w-13,29,fill='#506457')
   c.create_line(1,8,1,24,fill='#f4cb74',width=2)
   c.create_line(w-1,h-24,w-1,h-8,fill='#f4cb74',width=2)
  for panel in self.panels[len(self.records):]:panel.hide()


class ScanThrottle:
 def __init__(self):self.next_time=0
 def claim(self,now):
  if now<self.next_time:return False
  self.next_time=now+1
  return True






