import re
TOKEN=re.compile(r'(?<![\w.+-])([-+]?\d+\.\d{2,4})\s*(km|m)(?=\s|[-+\d]|$)',re.I)
def parse_units(text):
    text=text.replace(',','.').replace('−','-')
    text=re.sub(r'(km|m)(?=[-+\d])',r'\1 ',text,flags=re.I)
    matches=list(TOKEN.finditer(text))
    if len(matches)!=3:raise ValueError('Need three coordinate values with units')
    values=[]
    for m in matches:
        number,unit=m.groups(); decimals=len(number.split('.')[1])
        if decimals!=(4 if unit.lower()=='km' else 2):raise ValueError('Coordinate precision unclear')
        values.append(float(number)/(1000 if unit.lower()=='m' else 1))
    return tuple(values)


class UnitReader:
    def read_system(self,image):
        import cv2,numpy as np
        pixels=np.asarray(image.convert('RGB'))
        rows=[cv2.resize(pixels[y:y+12,-400:],None,fx=4,fy=4,interpolation=cv2.INTER_CUBIC)
              for y in range(171,289,13) if y+12<=image.height]
        if not rows:return None
        results,_=self.engine(rows)
        extra=getattr(self,'layout',None)
        adaptive=extra.read(image,'system') if extra else None
        if adaptive:results=list(results)+[adaptive]
        for text,score in results:
            if float(score)<.8:continue
            if not re.search(r'Current\s+player\s+location|SolarSystem|AsteroidCluster',text,re.I):continue
            match=re.search(r'(?<![a-z])(Nyx|Stanton|Pyro)(?=SolarSystem|[^a-z]|$)',text,re.I)
            if match:return match.group(1).capitalize()
        return None

    def read_shard(self,image):
        import cv2,numpy as np
        pixels=np.asarray(image.convert('RGB'))
        rows=[cv2.resize(pixels[y:y+15,-340:],None,fx=4,fy=4,interpolation=cv2.INTER_CUBIC)
              for y in (68,81,94) if y+15<=image.height]
        if not rows:return None
        results,_=self.engine(rows)
        extra=getattr(self,'layout',None)
        adaptive=extra.read(image,'shard') if extra else None
        if adaptive:results=list(results)+[adaptive]
        self.shard_evidence=[{'text':text,'confidence':float(score)} for text,score in results]
        for text,score in results:
            match=re.search(r'Shard[Iil1]d\s*:\s*(.+?)\s*$',text,re.I)
            if match and float(score)>=.85:
                # The recognizer often treats the thin underscores as spaces.
                value=re.sub(r'\s+','_',match.group(1))
                if re.fullmatch(r'[A-Za-z]+[_-][A-Za-z0-9]+[_-]\d{5,}[_-]\d{3,}',value):return value
        return None

    def read_scan_shard(self,image,known_ids=()):
        value=self.read_shard(image)
        if value:return value
        # A known ID is only recovered when the visible complete value corroborates
        # it repeatedly. Never reuse the cache solely because a read failed.
        import cv2,numpy as np
        pixels=np.asarray(image.convert('RGB'));votes={}
        normalize=lambda value:re.sub(r'[^a-z0-9]','',value.lower()).replace('s','5')
        known={normalize(value):value for value in known_ids if value and re.fullmatch(r'[A-Za-z]+[_-][A-Za-z0-9]+[_-]\d{5,}[_-]\d{3,}',value)}
        for y,height in [(82,13),(83,12),(84,11)]:
            for width in (177,171):
                crop=pixels[y:y+height,-width:]
                if not crop.size:continue
                gray=cv2.cvtColor(crop,cv2.COLOR_RGB2GRAY)
                rgb=cv2.cvtColor(gray,cv2.COLOR_GRAY2RGB)
                text,score=self.engine([cv2.resize(rgb,None,fx=4,fy=4)])[0][0]
                self.shard_evidence.append({'text':text,'confidence':float(score),'method':'value-only'})
                if score<.8:continue
                key=normalize(text)
                for expected,value in known.items():
                    if key==expected:votes[value]=votes.get(value,0)+1
        winners=[value for value,count in votes.items() if count>=3]
        return winners[0] if len(winners)==1 else None

    def __init__(self):
        from pathlib import Path
        import cv2,onnxruntime as ort
        import rapidocr_onnxruntime as package
        from rapidocr_onnxruntime import utils
        from rapidocr_onnxruntime.ch_ppocr_v3_rec import text_recognize
        cv2.setNumThreads(1)
        class BoundedSession(utils.OrtInferSession):
            def __init__(self,config):
                options=ort.SessionOptions()
                options.intra_op_num_threads=1
                options.inter_op_num_threads=1
                options.execution_mode=ort.ExecutionMode.ORT_SEQUENTIAL
                options.add_session_config_entry('session.intra_op.allow_spinning','0')
                options.add_session_config_entry('session.inter_op.allow_spinning','0')
                self.session=ort.InferenceSession(config['model_path'],sess_options=options,providers=['CPUExecutionProvider'])
        base=Path(package.__file__).parent
        config=utils.read_yaml(str(base/'config.yaml'))['Rec']
        config['model_path']=str(base/config['model_path'])
        original=text_recognize.OrtInferSession
        try:
            text_recognize.OrtInferSession=BoundedSession
            self.engine=text_recognize.TextRecognizer(config)
        finally:text_recognize.OrtInferSession=original
        actual=self.engine.session.session.get_session_options()
        assert actual.intra_op_num_threads==1 and actual.inter_op_num_threads==1
    def read(self,image):
        from debug_layout import DebugLayout
        if not hasattr(self,'layout'):self.layout=DebugLayout(self)
        if self.layout.boxes and self.layout.shape==image.size:
            try:return self.read_adaptive(image)
            except ValueError:pass
        # Retain the fast path for the original debug font/layout.
        try:return self.read_fixed(image)
        except ValueError:
            self.layout.discover(image)
            return self.read_adaptive(image)

    def read_adaptive(self,image):
        self.latest_orientation=None;positions=[];attempts=[]
        for key in ('camera','solar','root'):
            result=self.layout.read(image,key)
            if result is None:continue
            text,score=result;attempts.append({'row':key,'text':text,'confidence':float(score)})
            if key=='camera':
                match=re.search(r'Cam.{0,2}ir\s*:\s*(-?\d{1,3})\s+(-?\d{1,3})\s+(-?\d{1,3})(?=\s|F|$)',text,re.I)
                if match and score>=.75:
                    angles=tuple(map(int,match.groups()))
                    if all(-180<=v<=180 for v in angles):self.latest_orientation=angles
            elif score>=.82:
                try:positions.append(parse_units(text))
                except ValueError:pass
        self.evidence=attempts
        if len(positions)!=2 or any(abs(a-b)>.002 for a,b in zip(*positions)):
            raise ValueError('Adaptive coordinate rows unreadable or disagree: '+str(attempts))
        return tuple((a+b)/2 for a,b in zip(*positions)),attempts

    def read_fixed(self,image):
        import cv2,numpy as np
        pixels=np.asarray(image.convert('RGB'))
        self.latest_orientation=None
        attempts=[]
        # The expanded overlay adds CamDir ahead of the three Zone lines.
        expanded=image.height>=55
        if expanded:
            top=cv2.resize(pixels[4:16,-420:],None,fx=3,fy=3,interpolation=cv2.INTER_CUBIC)
            camera_result,_=self.engine([top])
            text,score=camera_result[0]
            attempts.append({'row':'camera','text':text,'confidence':float(score)})
            match=re.search(r'Cam[Dd0O]ir\s*:\s*(-?\d{1,3})\s+(-?\d{1,3})\s+(-?\d{1,3})(?=\s|F)',text,re.I)
            if match and float(score)>=.75:
                angles=tuple(map(int,match.groups()))
                if all(-180<=angle<=180 for angle in angles):self.latest_orientation=angles
            # Select layout from the text, not from whether the numbers parsed.
            expanded=bool(re.search(r'Cam.{0,2}ir',text,re.I))
        rows=[]
        starts=(30,43) if expanded else (17,30)
        for y in starts:
            rows.append(cv2.resize(pixels[y:y+12],None,fx=3,fy=3,interpolation=cv2.INTER_CUBIC))
        results,_=self.engine(rows)
        positions=[]
        for name,result in zip(('SolarSystem','Root'),results):
            text,score=result;confidence=float(score)
            attempts.append({'row':name,'text':text,'confidence':confidence})
            if confidence<.82:continue
            try:positions.append(parse_units(text))
            except ValueError:continue
        self.evidence=attempts
        if not positions:raise ValueError('Coordinate rows unreadable: '+str(attempts))
        if len(positions)==2:
            if any(abs(a-b)>.002 for a,b in zip(*positions)):
                raise ValueError('Printed coordinate rows disagree: '+str(attempts))
            return tuple((a+b)/2 for a,b in zip(*positions)),attempts
        return positions[0],attempts
