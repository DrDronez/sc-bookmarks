"""Discover debug rows only on layout changes/failures; reuse their bounds thereafter."""
import re,time

class DebugLayout:
 def __init__(self,reader):self.reader=reader;self.boxes={};self.shape=None;self.next_discovery=0;self.detector=None
 def discover(self,image):
  import cv2,numpy as np
  from pathlib import Path
  from rapidocr_onnxruntime import utils
  from rapidocr_onnxruntime.ch_ppocr_v3_det import text_detect
  import rapidocr_onnxruntime as package
  if image.size!=self.shape:self.boxes={};self.next_discovery=0;self.shape=image.size
  if time.monotonic()<self.next_discovery:return
  self.next_discovery=time.monotonic()+15
  if self.detector is None:
   base=Path(package.__file__).parent;cfg=utils.read_yaml(str(base/'config.yaml'))['Det'];cfg['model_path']=str(base/cfg['model_path'])
   original=text_detect.OrtInferSession;text_detect.OrtInferSession=type(self.reader.engine.session)
   try:self.detector=text_detect.TextDetector(cfg)
   finally:text_detect.OrtInferSession=original
  pixels=np.asarray(image.convert('RGB'));scale=min(1,1000/image.width)
  boxes,_=self.detector(cv2.resize(pixels,None,fx=scale,fy=scale))
  if boxes is None:return
  bands=[]
  for box in boxes:
   x,y=box.min(axis=0)/scale;xx,yy=box.max(axis=0)/scale
   if yy-y<5:continue
   match=next((b for b in bands if abs((b[1]+b[3]-y-yy)/2)<max(b[3]-b[1],yy-y)*.5),None)
   if match:match[:]=[min(match[0],x),min(match[1],y),max(match[2],xx),max(match[3],yy)]
   else:bands.append([x,y,xx,yy])
  found={};self.evidence=[]
  for x,y,xx,yy in sorted(bands,key=lambda b:b[1])[:32]:
   h=yy-y;bounds=(max(0,int(x-h*.2)),max(0,int(y-h*.15)),min(image.width,int(xx+h*.3)),min(image.height,int(yy+h*.2)))
   text,score=self.recognize(image,bounds)
   self.evidence.append((text,float(score),bounds))
   if score<.65:continue
   key=('root' if re.search(r'Zone\s*:\s*Root',text,re.I) else 'solar' if re.search(r'Zone\s*:\s*[S5]olar',text,re.I) else 'camera' if re.search(r'Cam.{0,2}ir\s*:',text,re.I) else 'shard' if re.search(r'Shard[Iil1]d',text,re.I) else 'system' if re.search(r'Current\s+player\s+location',text,re.I) else None)
   if key:found[key]=bounds
  if 'root' in found and 'solar' in found:self.boxes=found
 def recognize(self,image,box):
  import cv2,numpy as np
  crop=np.asarray(image.crop(box).convert('RGB'));scale=min(4,48/max(1,crop.shape[0]))
  return self.reader.engine([cv2.resize(crop,None,fx=scale,fy=scale,interpolation=cv2.INTER_CUBIC)])[0][0]
 def read(self,image,key):
  if image.size!=self.shape:self.boxes={}
  if key not in self.boxes:return None
  return self.recognize(image,self.boxes[key])


