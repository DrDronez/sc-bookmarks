"""Screenshot-only Star Citizen bookmark navigator. Independent observe-and-overlay application."""
import os
for variable in ('OMP_NUM_THREADS','OPENBLAS_NUM_THREADS','MKL_NUM_THREADS','NUMEXPR_NUM_THREADS'):
    os.environ[variable]='1'
from display_geometry import enable_dpi_awareness, capture_client_box, debug_box
enable_dpi_awareness()
import csv, json, math, queue, re, threading, time, winsound, ctypes, statistics
from ctypes import wintypes
from collections import deque
from pathlib import Path
import tkinter as tk
from tkinter import ttk, messagebox, simpledialog
from PIL import Image, ImageOps, ImageTk
from version import __version__

HOME = Path(__file__).resolve().parent
DATA = HOME / 'navigator-data'
OCR_LAST={}
GLYPH_READER=None
GLYPH_RAW_READER=None
FRESH_SECONDS=2.5
NUM = r'[-+]?\d+(?:\.\d+)?'
TRIPLE = re.compile(r'('+NUM+r')\s*km\s*('+NUM+r')\s*km\s*('+NUM+r')\s*km', re.I)
def parse_position(text):
    text=re.sub(r'(?<=\d)[lI](?=[\dkm.])','1',text)
    found={}
    for line in text.splitlines():
        if re.search(r'\bRoot\b',line,re.I): key='root'
        elif re.search(r'Solar\s*System',line,re.I): key='system'
        else: continue
        m=TRIPLE.search(line)
        if m: found[key]=m.groups()
    if 'root' in found and 'system' in found:
        a=list(found['root']);b=list(found['system'])
        for i in range(3):
            # Restore a missing decimal only when both printed rows have exactly
            # the same signed digit sequence and one retains its decimal point.
            if a[i].replace('.','')==b[i].replace('.',''):
                if '.' in a[i] and '.' not in b[i]:b[i]=a[i]
                elif '.' in b[i] and '.' not in a[i]:a[i]=b[i]
        if any('.' not in value or len(value.split('.')[-1])!=4 for value in a+b):
            raise ValueError('Coordinate precision is obscured; reading rejected')
        found['root']=tuple(map(float,a));found['system']=tuple(map(float,b))
        if norm(sub(found['root'],found['system'])) > .002:
            raise ValueError('Root/system disagree; reading rejected')
    # Both independently printed rows must agree before accepting a navigation sample.
    if len(found)!=2: raise ValueError('Need readable Root and SolarSystem rows')
    return found['root']
UNIT_READER=None
def read_coordinates(image):
    global OCR_LAST,UNIT_READER
    from unit_coordinates import UnitReader
    OCR_LAST={'method':'whole-line neural OCR / explicit units'}
    if UNIT_READER is None:UNIT_READER=UnitReader()
    position,evidence=UNIT_READER.read(image)
    OCR_LAST['rows']=evidence
    return position

AVERAGE_FRAMES=8
MAX_SAMPLE_AGE=8.0
AVERAGE_GAP=10.0
ESTIMATE_SECONDS=10.0

def estimate_position(position,stamp,velocity,now):
    """Constant-velocity estimate, held after the prediction horizon."""
    age=max(0,now-stamp)
    elapsed=min(age,ESTIMATE_SECONDS)
    return tuple(p+v*elapsed for p,v in zip(position,velocity)),age>ESTIMATE_SECONDS

class FrameAverage:
    def __init__(self):self.frames=deque(maxlen=AVERAGE_FRAMES)
    def add(self,t,p):
        if self.frames and t<=self.frames[-1][0]:return None
        if self.frames and t-self.frames[-1][0]>AVERAGE_GAP:self.frames.clear()
        if self.frames and norm(sub(p,self.frames[-1][1]))>400000*(t-self.frames[-1][0])+.1:
            raise ValueError('Implausible coordinate jump rejected')
        self.frames.append((t,p))
        if len(self.frames)<AVERAGE_FRAMES:return None
        # A bad digit must not contaminate all eight rolling averages.
        centers=[statistics.median(p[i] for t,p in self.frames) for i in range(3)]
        limits=[max(.02,6*statistics.median(abs(p[i]-centers[i]) for t,p in self.frames)) for i in range(3)]
        good=[(t,p) for t,p in self.frames if all(abs(p[i]-centers[i])<=limits[i] for i in range(3))]
        if len(good)<5:return None
        return (math.fsum(t for t,p in good)/len(good),
                tuple(math.fsum(p[i] for t,p in good)/len(good) for i in range(3)),good[-1][0])



def sub(a,b): return tuple(x-y for x,y in zip(a,b))
def dot(a,b): return sum(x*y for x,y in zip(a,b))
def norm(v): return math.sqrt(dot(v,v))
def unit(v):
    n=norm(v)
    if n<.001: raise ValueError('Move at least 1 metre for calibration')
    return tuple(x/n for x in v)
def cross(a,b): return (a[1]*b[2]-a[2]*b[1],a[2]*b[0]-a[0]*b[2],a[0]*b[1]-a[1]*b[0])
def approach(position,target,velocity):
    delta=sub(target,position); speed=norm(velocity)
    if speed<1e-8: return norm(delta),0,None,None
    t=dot(delta,velocity)/dot(velocity,velocity)
    miss=norm(tuple(x-t*v for x,v in zip(delta,velocity)))
    closing=dot(delta,velocity)/max(norm(delta),1e-12)
    return norm(delta),closing,t,miss
def fit_velocity(samples):
    if len(samples)<3 or samples[-1][0]-samples[0][0]<.4: return (0,0,0)
    ts=[t-samples[0][0] for t,p in samples]; mean=sum(ts)/len(ts)
    den=sum((t-mean)**2 for t in ts)
    return tuple(sum((t-mean)*(p[i]-samples[0][1][i]) for t,(_,p) in zip(ts,samples))/den for i in range(3))

def duration_text(seconds):
    seconds=max(0,math.ceil(seconds))
    hours,remainder=divmod(seconds,3600);minutes,seconds=divmod(remainder,60)
    return f'{hours}h {minutes:02d}m' if hours else f'{minutes:02d}:{seconds:02d}'

class TravelMotion:
    """Infer high-speed travel from recent pixels, independent of long position averaging."""
    def __init__(self):
        self.samples=deque(maxlen=4);self.quantum=False;self.velocity=(0,0,0)
    def add(self,t,p):
        if self.samples and t<=self.samples[-1][0]:return
        if self.samples and t-self.samples[-1][0]>FRESH_SECONDS:
            self.samples.clear();self.velocity=(0,0,0)
        self.samples.append((t,p))
        segments=[tuple((b-a)/(tb-ta) for a,b in zip(pa,pb))
                  for (ta,pa),(tb,pb) in zip(self.samples,list(self.samples)[1:])]
        if len(segments)<2:return
        self.velocity=tuple(statistics.median(v[i] for v in segments) for i in range(3))
        speeds=[norm(v) for v in segments[-2:]]
        if all(s>5 for s in speeds) and dot(segments[-1],segments[-2])>0:self.quantum=True
        elif all(s<2 for s in speeds):self.quantum=False
    def fresh(self,now):
        return len(self.samples)>=3 and now-self.samples[-1][0]<=FRESH_SECONDS
    def position(self,now):
        t,p=self.samples[-1]
        return estimate_position(p,t,self.velocity,now)[0]

def arrival_text(distance,closing,speed,fresh):
    if not fresh:return 'ETA paused — waiting for fresh position'
    if distance<=.02:return 'AT BOOKMARK (within 20 m)'
    if speed<.005:return 'ETA — stopped'
    if closing<-.005:return 'ETA — moving away'
    if closing<=.005:return 'ETA — not approaching'
    return 'ETA ~'+duration_text(distance/closing)+' at current closing speed'
def steering(delta,forward,right):
    f=unit(forward); r0=unit(right)
    r=unit(tuple(x-dot(r0,f)*y for x,y in zip(r0,f)))
    # In the observed game coordinate handedness, right x forward points up.
    u=unit(cross(r,f)); x,y,z=dot(delta,r),dot(delta,u),dot(delta,f)
    return math.degrees(math.atan2(x,z)),math.degrees(math.atan2(y,math.hypot(x,z)))

def angle_between(a,b):
    return math.degrees(math.acos(max(-1,min(1,dot(a,b)/(norm(a)*norm(b))))))

def transport_right(old_forward,old_right,new_forward):
    """Minimum-rotation frame transport; cannot infer roll from translation."""
    f=unit(old_forward);g=unit(new_forward);c=max(-1,min(1,dot(f,g)))
    if c<-.5:raise ValueError('Large direction jump: recalibration required')
    axis=cross(f,g);s=norm(axis)
    if s<1e-8:return unit(old_right)
    a=tuple(x/s for x in axis);r=old_right;k=cross(a,r)
    result=tuple(x*c+y*s+z*dot(a,r)*(1-c) for x,y,z in zip(r,k,a))
    return unit(tuple(x-dot(result,g)*y for x,y in zip(result,g)))

class CourseGuide:
    def __init__(self):self.reset()
    def reset(self):
        self.phase='baseline';self.stable=deque(maxlen=6);self.baseline=None
        self.forward=None;self.right=None;self.last_time=None
        self.heading='1/2  FLY STRAIGHT';self.detail='Fly forward at 20 m/s or more';self.yaw=None;self.pitch=None
    def update(self,t,velocity,delta):
        self.yaw=None;self.pitch=None;speed=norm(velocity)
        if speed>2:
            if self.phase!='baseline' or self.stable:self.reset()
            self.heading='QT / HIGH-SPEED TRAVEL';self.detail='Setup starts automatically after QT';return
        if speed<.02:
            self.heading='FLY FORWARD TO RESUME' if self.phase=='tracking' else ('1/2  FLY STRAIGHT' if self.phase=='baseline' else '2/2  SMALL RIGHT TURN')
            self.detail='Calibration kept; move forward at 20 m/s or more' if self.phase=='tracking' else 'Fly forward at 20 m/s or more'
            self.stable.clear();self.last_time=t;return
        f=unit(velocity);self.last_time=t
        if self.phase=='tracking':
            try:self.right=transport_right(self.forward,self.right,f)
            except ValueError:
                self.heading='HOLD A FORWARD COURSE';self.detail='Direction changed sharply; waiting for a stable reading';return
            self.forward=f
            self.yaw,self.pitch=steering(delta,f,self.right)
            self.heading='FOLLOW THE ARROW';self.detail='Calibration kept while stopped or through OCR gaps'
            return
        if self.stable and angle_between(self.stable[-1][1],f)>4:self.stable.clear()
        self.stable.append((t,f))
        span=t-self.stable[0][0]
        steady=len(self.stable)>=3 and span>=4
        progress=min(100,round(min(span/4,len(self.stable)/3)*100))
        if self.phase=='baseline':
            self.heading='1/2  FLY STRAIGHT'
            self.detail=f'Keep the nose still — learning direction {progress}%'
            if steady:
                self.baseline=unit(tuple(sum(v[i] for _,v in self.stable) for i in range(3)))
                self.phase='right_probe';self.stable.clear()
                self.heading='2/2  SMALL RIGHT TURN';self.detail='Yaw right about 10 degrees, then keep flying straight'
            return
        amount=angle_between(self.baseline,f)
        self.heading='2/2  SMALL RIGHT TURN'
        if amount<4:
            self.stable.clear();self.detail='Yaw right about 10 degrees, then hold the nose still';return
        if amount>60:
            self.detail='Turn back closer to your initial heading; then make a small right turn';return
        self.detail=f'Hold this course — finishing setup {progress}%'
        if steady:
            c=dot(self.baseline,f)
            self.right=unit(tuple(c*x-y for x,y in zip(f,self.baseline)))
            self.forward=f;self.phase='tracking';self.stable.clear()
            self.yaw,self.pitch=steering(delta,f,self.right)
            self.heading='FOLLOW THE ARROW';self.detail='Setup complete; keep flying forward'

def camera_basis(angles):
    # Validated against forward travel: Rz(z) Ry(y) Rx(x), forward = +Y.
    a,b,c=map(math.radians,angles)
    sa,ca=math.sin(a),math.cos(a);sb,cb=math.sin(b),math.cos(b);sc,cc=math.sin(c),math.cos(c)
    right=(cc*cb,sc*cb,-sb)
    forward=(cc*sb*sa-sc*ca,sc*sb*sa+cc*ca,cb*sa)
    up=(cc*sb*ca+sc*sa,sc*sb*ca-cc*sa,cb*ca)
    return forward,right,up

class CameraGuide:
    def __init__(self):self.reset()
    def reset(self):
        self.phase='camera';self.yaw=None;self.pitch=None
        self.heading='WAITING FOR CAMDIR';self.detail='Keep the cockpit camera centred'
    def update_angles(self,angles,delta):
        forward,right,up=camera_basis(angles)
        self.yaw=math.degrees(math.atan2(dot(delta,right),dot(delta,forward)))
        self.pitch=math.degrees(math.atan2(dot(delta,up),math.hypot(dot(delta,right),dot(delta,forward))))
        self.heading='AIM TOWARD THE BOOKMARK';self.detail='CamDir guidance — works while stopped; angles approximate'

class FlightOverlay:
    def __init__(self,root):
        from overlay_settings import Settings
        self.settings=Settings()
        self.window=tk.Toplevel(root);self.window.withdraw();self.window.title('SC Bookmarks overlay')
        self.window.overrideredirect(True);self.window.attributes('-topmost',True)
        self.key='#010203';self.window.configure(bg=self.key);self.window.attributes('-transparentcolor',self.key)
        self.canvas=tk.Canvas(self.window,width=460,height=265,bg=self.key,highlightthickness=0);self.canvas.pack()
        self.window.update_idletasks();self.visible=False
        user=ctypes.WinDLL('user32',use_last_error=True);self.user=user
        user.GetAncestor.argtypes=[wintypes.HWND,wintypes.UINT];user.GetAncestor.restype=wintypes.HWND
        user.GetWindowLongW.argtypes=[wintypes.HWND,ctypes.c_int];user.GetWindowLongW.restype=ctypes.c_long
        user.SetWindowLongW.argtypes=[wintypes.HWND,ctypes.c_int,ctypes.c_long];user.SetWindowLongW.restype=ctypes.c_long
        user.SetWindowPos.argtypes=[wintypes.HWND,wintypes.HWND,ctypes.c_int,ctypes.c_int,ctypes.c_int,ctypes.c_int,wintypes.UINT]
        user.FindWindowW.argtypes=[wintypes.LPCWSTR,wintypes.LPCWSTR];user.FindWindowW.restype=wintypes.HWND
        user.GetWindowRect.argtypes=[wintypes.HWND,ctypes.POINTER(wintypes.RECT)]
        user.GetClientRect.argtypes=[wintypes.HWND,ctypes.POINTER(wintypes.RECT)]
        user.ClientToScreen.argtypes=[wintypes.HWND,ctypes.POINTER(wintypes.POINT)]
        user.IsIconic.argtypes=[wintypes.HWND]
        self.hwnd=user.GetAncestor(self.window.winfo_id(),2)
        flags=user.GetWindowLongW(self.hwnd,-20)|0x80000|0x20|0x08000000|0x80
        user.SetWindowLongW(self.hwnd,-20,flags)
    def hide(self):
        if self.visible:self.window.withdraw();self.visible=False
    def game_rect(self):
        game=self.user.FindWindowW(None,'Star Citizen ') or self.user.FindWindowW(None,'Star Citizen')
        if not game or self.user.IsIconic(game):return None
        rect=wintypes.RECT();origin=wintypes.POINT(0,0)
        if not self.user.GetClientRect(game,ctypes.byref(rect)) or not self.user.ClientToScreen(game,ctypes.byref(origin)):return None
        width=rect.right-rect.left;height=rect.bottom-rect.top
        if width<=0 or height<=0:return None
        return origin.x,origin.y,width,height
    def text(self,y,text,size=13,color='#e4f6ff'):
        # Pixel-sized fonts stay inside the fixed canvas at differing Windows DPI settings.
        font=('Segoe UI',-round(size*96/72),'bold')
        for dx,dy in [(-1,0),(1,0),(0,-1),(0,1),(2,2)]:
            self.canvas.create_text(230+dx,y+dy,text=text,font=font,fill='#000000')
        self.canvas.create_text(230,y,text=text,font=font,fill=color)
    def render(self,target,distance,guide,fresh,enabled,angle=None,held=False,orientation_fresh=False,quantum=False,travel_lines=()):
        if not target or not enabled:self.hide();return
        from overlay_settings import placement as overlay_placement
        rect=self.game_rect()
        if rect is None:self.hide();return
        placement=overlay_placement(rect,self.settings.current)
        x,y,w,h=placement
        # Show without activation; mouse input passes through the layered window.
        if not self.visible:self.window.deiconify();self.visible=True
        if getattr(self,'placement',None)!=placement:
            self.user.SetWindowPos(self.hwnd,ctypes.c_void_p(-1),x,y,w,h,0x10|0x40|0x4000)
            self.placement=placement
        c=self.canvas;c.delete('all')
        self.text(17,target[:48],12,'#a8cce0')
        if distance is None:self.text(47,'WAITING FOR POSITION',19,'#ffca65')
        else:
            value=f'{distance:,.2f} km' if distance>=1 else f'{distance*1000:,.0f} m'
            self.text(47,('HELD: ' if held else ('' if fresh else 'EST: '))+value,21,'#63ecff' if fresh else '#ffca65')
        if quantum:
            self.text(78,'QUANTUM TRAVEL (speed detected)',11,'#ffcf70')
            for y,line in zip((110,145,180,214),travel_lines):self.text(y,line,12,'#ffca65')
            return
        if travel_lines:self.text(190,travel_lines[0],10,'#a8f3da')
        if not fresh and not orientation_fresh:
            self.text(100,'COORDINATES UNREADABLE — GUIDANCE PAUSED',12,'#ff6969')
            self.text(137,'Reader retrying on the next game frame',10,'#ffca65');return
        self.text(78,guide.heading,11,'#ffcf70' if guide.phase!='tracking' else '#a8f3da')
        if guide.phase=='right_probe' and guide.yaw is None:
            c.create_line(185,121,277,121,fill='#ffca65',width=6,arrow='last',arrowshape=(16,19,7))
            self.text(156,'RIGHT TURN TO CALIBRATE',11,'#ffca65')
        elif guide.yaw is not None:
            yaw,pitch=guide.yaw,guide.pitch
            if abs(yaw)<2 and abs(pitch)<2:
                c.create_oval(214,104,246,136,outline='#78ffac',width=3);self.text(158,'ON COURSE',12,'#78ffac')
            else:
                dx=max(-1,min(1,yaw/20));dy=-max(-1,min(1,pitch/20))
                length=math.hypot(dx,dy)
                if length:dx=dx/length*45;dy=dy/length*36
                c.create_line(230,120,230+dx,120+dy,fill='#63ecff',width=6,arrow='last',arrowshape=(16,19,7))
                self.text(158,f'{abs(yaw):.1f}° '+('RIGHT' if yaw>=0 else 'LEFT')+f'   {abs(pitch):.1f}° '+('UP' if pitch>=0 else 'DOWN'),12)
        else:self.text(122,f'{angle:.1f}° off travel direction' if angle is not None else 'Waiting for forward movement',13)

class Navigator:
    def __init__(self,root,auto_capture=True):
        self.root=root; root.title('SC Bookmarks '+__version__); root.geometry('750x620')
        self.q=queue.Queue(); self.stop=threading.Event(); self.position=None; self.last_good=0;self.position_time=0
        self.asteroid_request=threading.Event();self.asteroid_busy=False
        self.samples=deque(maxlen=12); self.bookmarks=[]; self.target=None; self.armed=False; self.alerted=False
        self.basis={}; self.cal_start=None; self.running=False; self.last_logged=0
        self.course=CameraGuide();self.camdir=None;self.camdir_time=0;self.overlay=FlightOverlay(root)
        from scan_contacts import ContactOverlay
        self.contact_overlay=ContactOverlay(root,FlightOverlay)
        self.contact_enabled=threading.Event();self.contact_enabled.set()
        self.contact_busy=threading.Event();self.scan_reader=None
        self.scan_labels_enabled=tk.BooleanVar(value=True)
        self.scan_label_status=tk.StringVar(value='RS labels: fast badge checks; periodic discovery while capture runs.')
        self.scan_labels_enabled.trace_add('write',lambda *a:self.contact_enabled.set() if self.scan_labels_enabled.get() else self.contact_enabled.clear())
        self.motion=TravelMotion();self.qt_seen_ahead=False
        self.shard_id=None;self.shard_candidate=None;self.shard_time=0;self.shard_candidate_time=0
        self.system_name=tk.StringVar(value='Unknown')
        self.category_filter=tk.StringVar(value='All categories');self.new_category=tk.StringVar(value='Personal')
        self.hide_other_systems=tk.BooleanVar(value=False);self.sort_order=tk.StringVar(value='Category')
        self.auto_system=tk.BooleanVar(value=True)
        self.ore_filter=tk.StringVar(value='Any ore');self.quality_filter=tk.StringVar(value='')
        self.applied_ore='';self.applied_quality=None
        self.shard_text=tk.StringVar(value='Current shard: checking…')
        self.bookmark_details=tk.StringVar(value='')
        self.overlay_enabled=tk.BooleanVar(value=True);self.motion_angle=None
        self.status=tk.StringVar(value='Stopped. Start capture while the game overlay is visible.')
        self.pos_text=tk.StringVar(value='Root XYZ km:\nWaiting for game coordinates…'); self.nav_text=tk.StringVar(value='Choose a bookmark to navigate.')
        self.steer_text=tk.StringVar(value='Orientation requires a fresh calibration at your current attitude.')
        self.name=tk.StringVar(value='New space bookmark'); self.lead=tk.StringVar(value='3.0')
        self.capture_cfg=None
        self.preview_image=None;self.preview_window=None;self.source_text=tk.StringVar(value='Source: Star Citizen window (works without focus)')
        self.pending_bookmark=None;self.pending_name=None
        self.countdown_text=tk.StringVar(value='Timed bookmark: click below, then return to the game.')
        self.reading_age=tk.StringVar(value='No coordinate reading captured yet.')
        DATA.mkdir(exist_ok=True)
        if (DATA/'bookmarks.json').exists(): self.bookmarks=json.loads((DATA/'bookmarks.json').read_text())
        from bookmark_links import DEFAULT_CATEGORIES
        try:settings=json.loads((DATA/'organization.json').read_text())
        except (OSError,ValueError):settings={}
        self.categories=settings.get('categories',list(DEFAULT_CATEGORIES))
        if 'Mining' not in self.categories:self.categories.insert(0,'Mining')
        for b in self.bookmarks:
            b.setdefault('category','Personal')
            if b['category'] not in self.categories:self.categories.append(b['category'])
        self.system_name.set(settings.get('system','Unknown'))
        self.hide_other_systems.set(settings.get('hide_other_systems',False))
        self.sort_order.set(settings.get('sort','Category'))
        self.auto_system.set(settings.get('auto_system',True))
        self.save()
        from navigator_ui import build
        build(self)
        root.protocol('WM_DELETE_WINDOW',self.close); root.after(100,self.poll)
        if auto_capture:root.after(500,self.start)
    def schedule_bookmark(self):
        if not self.name.get().strip():return messagebox.showinfo('Bookmark','Enter a bookmark name first.')
        if not self.running:self.start()
        if not self.running:return
        self.pending_name=self.name.get().strip();self.pending_bookmark=time.monotonic()+8
        self.countdown_text.set('Return to the game now. Saving in 8 seconds…')
    def cancel_bookmark(self):
        self.pending_bookmark=None;self.pending_name=None;self.countdown_text.set('Bookmark timer cancelled.')
    def save(self):
        path=DATA/'bookmarks.json'; tmp=path.with_suffix('.tmp'); tmp.write_text(json.dumps(self.bookmarks,indent=2));tmp.replace(path)
    def save_organization(self):
        settings={'categories':self.categories,'system':self.system_name.get(),
                  'hide_other_systems':self.hide_other_systems.get(),'sort':self.sort_order.get(),'auto_system':self.auto_system.get()}
        path=DATA/'organization.json';tmp=path.with_suffix('.tmp')
        try:tmp.write_text(json.dumps(settings,indent=2));tmp.replace(path)
        except OSError as e:messagebox.showerror('Save organization',str(e),parent=self.root)
    def organization_changed(self):
        self.refresh_bookmarks();self.save_organization()
    def refresh_bookmarks(self,selected=None):
        from bookmark_links import organize
        if selected is None and hasattr(self,'visible_bookmarks'):
            indexes=self.list.curselection()
            if indexes:selected=self.visible_bookmarks[indexes[0]]
        self.visible_bookmarks=organize(self.bookmarks,self.category_filter.get(),self.system_name.get(),
                                       self.hide_other_systems.get(),self.sort_order.get(),self.categories)
        from mining_bookmarks import filter_mining,best_quality
        self.visible_bookmarks=filter_mining(self.visible_bookmarks,self.applied_ore,self.applied_quality,self.sort_order.get())
        self.list.delete(0,'end')
        for i,b in enumerate(self.visible_bookmarks):
            label=f"[{b.get('category','Personal')}] {b['name']} — {b.get('system','unknown')}"
            if b.get('category')=='Mining':
                quality=best_quality(b,self.applied_ore)
                if quality>=0:label+=f" | {self.applied_ore or 'Best'} Q{quality:g}"
            self.list.insert('end',label)
            if b is selected:self.list.selection_set(i)
        self.category_box.configure(values=['All categories']+self.categories)
        self.destination_box.configure(values=self.categories)
    def focus_bookmark(self,bookmark):
        self.refresh_bookmarks(bookmark)
        if any(b is bookmark for b in self.visible_bookmarks):self.select()
    def move_category(self):
        indexes=self.list.curselection()
        if not indexes:return messagebox.showinfo('Move bookmark','Select a bookmark first.',parent=self.root)
        b=self.visible_bookmarks[indexes[0]];old=b.get('category','Personal');b['category']=self.new_category.get()
        try:self.save()
        except OSError as e:
            b['category']=old;return messagebox.showerror('Move bookmark',str(e),parent=self.root)
        self.refresh_bookmarks(b)
    def manage_categories(self):
        app=self
        class CategoriesDialog(simpledialog.Dialog):
            def body(self,master):
                self.names=list(app.categories);self.renames={n:n for n in self.names}
                ttk.Label(master,text='Mining is fixed. Other categories can be renamed; all can be reordered.').pack()
                self.items=tk.Listbox(master,width=40,height=9,exportselection=False);self.items.pack(fill='x')
                row=ttk.Frame(master);row.pack(pady=5)
                for text,command in [('Add',self.add),('Rename',self.rename),('Up',lambda:self.move(-1)),('Down',lambda:self.move(1))]:
                    ttk.Button(row,text=text,command=command).pack(side='left')
                self.refresh();return self.items
            def refresh(self,index=0):
                self.items.delete(0,'end')
                for name in self.names:self.items.insert('end',name)
                if self.names:self.items.selection_set(index)
            def ask_name(self,initial=''):
                name=simpledialog.askstring('Category','Category name:',initialvalue=initial,parent=self)
                if name is None:return None
                name=name.strip()
                if not name or len(name)>80 or name.casefold()=='all categories' or any(n.casefold()==name.casefold() and n!=initial for n in self.names):
                    messagebox.showerror('Category','Use a unique category name, 1–80 characters.',parent=self);return None
                return name
            def add(self):
                name=self.ask_name()
                if name:self.names.append(name);self.refresh(len(self.names)-1)
            def rename(self):
                selected=self.items.curselection()
                if not selected:return
                i=selected[0];old=self.names[i]
                if old=='Mining':return messagebox.showinfo('Mining category','Mining is a fixed category and cannot be renamed.',parent=self)
                name=self.ask_name(old)
                if name:
                    self.names[i]=name
                    self.renames={k:name if v==old else v for k,v in self.renames.items()}
                    self.refresh(i)
            def move(self,step):
                selected=self.items.curselection()
                if not selected:return
                i=selected[0];j=i+step
                if 0<=j<len(self.names):self.names[i],self.names[j]=self.names[j],self.names[i];self.refresh(j)
            def validate(self):
                old=[b.get('category','Personal') for b in app.bookmarks]
                for b in app.bookmarks:b['category']=self.renames.get(b.get('category','Personal'),b.get('category','Personal'))
                try:app.save()
                except OSError as e:
                    for b,c in zip(app.bookmarks,old):b['category']=c
                    messagebox.showerror('Save categories',str(e),parent=self);return False
                app.categories=self.names
                app.category_filter.set(self.renames.get(app.category_filter.get(),app.category_filter.get()))
                app.new_category.set(self.renames.get(app.new_category.get(),app.new_category.get()))
                app.organization_changed();return True
        CategoriesDialog(self.root,title='Manage categories')
    def current_shard(self):
        return self.shard_id if time.monotonic()-self.shard_time<=45 else None
    def add_bookmarks(self,bookmarks):
        for bookmark in bookmarks:
            if bookmark.get('kind')!='imported bookmark':
                bookmark['utc']=time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime())
        original=list(self.bookmarks)
        self.bookmarks.extend(bookmarks)
        try:self.save()
        except OSError:
            self.bookmarks=original;raise
        for b in bookmarks:
            if b.get('category','Personal') not in self.categories:self.categories.append(b.get('category','Personal'))
        self.refresh_bookmarks();self.save_organization()
    def copy_bookmarks(self,all_bookmarks=False):
        from bookmark_links import encode
        selected=self.list.curselection()
        if not all_bookmarks and not selected:
            return messagebox.showinfo('Copy bookmark','Select a bookmark first.',parent=self.root)
        items=self.bookmarks if all_bookmarks else [self.visible_bookmarks[selected[0]]]
        try:text='\n'.join(encode(b) for b in items)
        except ValueError as e:return messagebox.showerror('Copy bookmark',str(e),parent=self.root)
        self.root.clipboard_clear();self.root.clipboard_append(text)
        self.bookmark_details.set(f'Copied {len(items)} bookmark link(s) to the clipboard.')
    def import_bookmarks(self):
        from bookmark_links import decode_many
        app=self
        class ImportDialog(simpledialog.Dialog):
            def body(self,master):
                ttk.Label(master,text='Paste SC Bookmarks links, one per line. Shards are informational only.').pack(anchor='w')
                self.text=tk.Text(master,width=80,height=9,wrap='word');self.text.pack()
                try:
                    value=app.root.clipboard_get()
                    if value.strip().startswith('scbm://'):self.text.insert('1.0',value[:4000000])
                except tk.TclError:pass
                return self.text
            def validate(self):
                try:
                    bookmarks=decode_many(self.text.get('1.0','end'))
                    app.add_bookmarks(bookmarks)
                    app.bookmark_details.set(f'Imported {len(bookmarks)} bookmark(s). Select one to navigate.')
                    return True
                except (ValueError,OSError) as e:
                    messagebox.showerror('Import bookmarks',str(e),parent=self);return False
        ImportDialog(self.root,title='Paste / import bookmarks')
    def manual_bookmark(self):
        from bookmark_editor import BookmarkEditor
        b={'name':'','system':self.system_name.get(),'xyz_km':['','',''],'kind':'manual bookmark',
           'category':self.new_category.get(),'utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime())}
        def save_new(candidate):
            self.add_bookmarks([candidate]);self.refresh_bookmarks(candidate)
            self.bookmark_details.set('Added '+candidate['name'])
        BookmarkEditor(self.root,b,self.categories,save_new)
    def edit_bookmark(self):
        from bookmark_editor import BookmarkEditor
        selected=self.list.curselection()
        if not selected:return messagebox.showinfo('Bookmark details','Select a bookmark first.',parent=self.root)
        bookmark=self.visible_bookmarks[selected[0]]
        def save_edit(candidate):
            original=dict(bookmark);bookmark.clear();bookmark.update(candidate)
            try:self.save()
            except OSError:
                bookmark.clear();bookmark.update(original);raise
            self.refresh_bookmarks(bookmark)
            self.bookmark_details.set('Saved '+bookmark['name']+(' — '+bookmark['notes'][:180] if bookmark.get('notes') else ''))
            if bookmark is self.target:self.alerted=False;self.qt_seen_ahead=False
        BookmarkEditor(self.root,bookmark,self.categories,save_edit)
    def apply_mining_filter(self):
        from mining_bookmarks import number
        try:minimum=number(self.quality_filter.get(),'Minimum quality')
        except ValueError as e:return messagebox.showerror('Find rocks',str(e),parent=self.root)
        self.applied_ore='' if self.ore_filter.get()=='Any ore' else self.ore_filter.get()
        self.applied_quality=minimum
        self.category_filter.set('Mining');self.sort_order.set('Ore quality');self.refresh_bookmarks()
    def clear_mining_filter(self):
        self.applied_ore='';self.applied_quality=None;self.ore_filter.set('Any ore');self.quality_filter.set('')
        self.refresh_bookmarks()
    def fresh(self):
        if self.position is None or time.monotonic()-self.last_good>FRESH_SECONDS: raise ValueError('No fresh coordinate reading. Keep the overlay visible.')
        return self.position
    def bookmark_asteroid(self):
        if self.asteroid_busy:return
        if not self.running:self.start()
        self.asteroid_busy=True;self.asteroid_request.set()
        self.status.set('Capturing asteroid scan… keep the selected rock centered.')
        def timeout():
            if self.asteroid_request.is_set():
                self.asteroid_request.clear();self.asteroid_busy=False
                messagebox.showerror('Bookmark asteroid','No game frame arrived. Restore the game window and try again.',parent=self.root)
        self.root.after(10000,timeout)
    def read_asteroid(self,image):
        reader=None
        try:
            image.save(DATA/'last-asteroid-capture.png')
            from unit_coordinates import UnitReader
            from asteroid_scan import read_scan
            reader=UnitReader()
            strip=image.crop(debug_box(image.width,image.height))
            position,_=reader.read(strip);angles=reader.latest_orientation
            if angles is None:raise ValueError('CamDir is unreadable. Keep the expanded coordinate overlay visible.')
            mining,distance,evidence=read_scan(image,reader)
            shard=reader.read_scan_shard(strip,tuple(b.get('shard_id') for b in self.bookmarks))
            self.q.put(('asteroid',position,angles,mining,distance,shard))
        except Exception as e:
            try:(DATA/'last-asteroid-error.json').write_text(json.dumps({'error':str(e),'rows':getattr(reader,'scan_evidence',[])},indent=2))
            except OSError:pass
            self.q.put(('asteroid_error',str(e)))
    def review_asteroid(self,position,angles,mining,distance,shard=None):
        from bookmark_editor import BookmarkEditor
        distance=simpledialog.askfloat('Asteroid distance',
            'Check the selected rock distance (metres).\nPosition assumes the rock is centered in your aim.\nCamDir is rounded, so this is an approximate location.',
            initialvalue=distance,minvalue=0,maxvalue=1000000,parent=self.root)
        if distance is None:return
        forward,_,_=camera_basis(angles)
        xyz=[p+v*distance/1000 for p,v in zip(position,forward)]
        ores=list(dict.fromkeys(row['ore'] for row in mining['composition']))
        bookmark={'name':' / '.join(ores)+' asteroid','system':self.system_name.get(),
                  'xyz_km':xyz,'category':'Mining','mining':mining,'shard_id':shard,
                  'utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime()),'kind':'scanned asteroid',
                  'notes':f'Approximate position from ship + {distance:g} m along camera aim. Review OCR values before saving.',
                  'scan_ship_xyz_km':list(position),'scan_camdir':list(angles),'scan_distance_m':distance}
        def save(candidate):
            if not candidate.get('shard_id'):
                raise ValueError('Shard ID was not readable in the capture. Enter it on the Bookmark & notes tab before saving this asteroid.')
            self.add_bookmarks([candidate]);self.refresh_bookmarks(candidate)
            self.status.set('Saved asteroid: '+candidate['name'])
        if not shard:
            messagebox.showwarning('Shard ID unreadable',
                'The asteroid scan was read, but Shard ID was obscured or unreadable.\nEnter the current Shard ID on the Bookmark & notes tab before saving, or cancel and capture again with that line clear.',parent=self.root)
        BookmarkEditor(self.root,bookmark,self.categories,save)
    def show_preview(self):
        if self.preview_window is None or not self.preview_window.winfo_exists():
            self.preview_window=tk.Toplevel(self.root);self.preview_window.title('Actual game-window capture')
            ttk.Label(self.preview_window,text='Original game pixels').pack(anchor='w',padx=8,pady=(6,0))
            self.preview_label=ttk.Label(self.preview_window);self.preview_label.pack(padx=8,pady=8)
            ttk.Label(self.preview_window,text='White/neutral text mask used by the reader').pack(anchor='w',padx=8)
            self.mask_label=ttk.Label(self.preview_window);self.mask_label.pack(padx=8,pady=8)
            ttk.Label(self.preview_window,text='Colored HUD pixels are suppressed; white/gray interference can remain.').pack(padx=8,pady=8)
        if self.preview_image is not None:
            self.preview_photo=ImageTk.PhotoImage(self.preview_image)
            self.preview_label.configure(image=self.preview_photo)
            if GLYPH_READER is not None:
                import numpy as np
                mask=GLYPH_READER.signal(np.asarray(self.preview_image.convert('RGB')))
                self.mask_photo=ImageTk.PhotoImage(Image.fromarray((mask*255).astype('uint8')))
                self.mask_label.configure(image=self.mask_photo)
    def start(self):
        if self.running:return
        bbox=None
        self.status.set('Starting game capture…');self.scan_label_status.set('RS labels: starting capture…')
        self.stop.clear();self.running=True;self.samples.clear();self.capture_cfg=bbox;self.motion=TravelMotion()
        threading.Thread(target=self.worker,args=(bbox,),daemon=True).start()
    def halt(self):self.stop.set();self.status.set('Stopping capture…')
    def read_contacts(self,image,stamp,angles):
        try:
            from scan_contacts import ScanReader
            if self.scan_reader is None:self.scan_reader=ScanReader()
            records=self.scan_reader.read(image)
            (DATA/'scan-status.json').write_text(json.dumps({'utc':time.time(),'snapshot_age':time.monotonic()-stamp,'scan_mode':self.scan_reader.mode,'contacts':[{'rs':r['rs'],'text':r['text'],'bounds':r.get('bounds'),'anchor':[r['x'],r['y']]} for r in records],'candidates':self.scan_reader.evidence},indent=2),encoding='utf-8')
            self.q.put(('contacts',stamp,image.size,records,angles))
        except Exception as e:self.q.put(('contact_error','RS labels: '+str(e)[:120]))
        finally:self.contact_busy.clear()
    def worker(self,bbox):
        control=None
        from scan_contacts import ScanThrottle
        scan_throttle=ScanThrottle()
        frames=queue.Queue(maxsize=1);closed=threading.Event();last_frame=[0.0]
        good_count=0;bad_count=0;average=FrameAverage();next_shard_read=0;previous_shard=None;next_system_read=0
        try:
            from capture_process import CaptureProcess
            def on_frame_arrived(frame,capture_control):
                stamp=frame.stamp
                if stamp-last_frame[0]<.45:return
                last_frame[0]=stamp
                try:
                    cx,cy,cr,cb=capture_client_box(frame.width,frame.height)
                    if self.contact_enabled.is_set() and not self.contact_busy.is_set() and not self.asteroid_busy and scan_throttle.claim(stamp):
                        self.contact_busy.set()
                        scan_image=Image.fromarray(frame.frame_buffer[cy:cb,cx:cr,:3][:,:,::-1].copy())
                        threading.Thread(target=self.read_contacts,args=(scan_image,stamp,self.camdir),daemon=True).start()
                    if self.asteroid_request.is_set():
                        self.asteroid_request.clear()
                        shot=Image.fromarray(frame.frame_buffer[cy:cb,cx:cr,:3][:,:,::-1].copy())
                        threading.Thread(target=self.read_asteroid,args=(shot,),daemon=True).start()
                    left,top,right,bottom=bbox or debug_box(cr-cx,cb-cy)
                    left+=cx;right+=cx;top+=cy;bottom+=cy
                    if right>frame.width or bottom>frame.height:raise ValueError('Crop exceeds the game window dimensions')
                    # Copy only the OCR strip; never retain the native full-frame buffer.
                    pixels=frame.frame_buffer[top:bottom,left:right,:3][:,:,::-1].copy()
                    image=Image.fromarray(pixels)
                    try:frames.get_nowait()
                    except queue.Empty:pass
                    frames.put_nowait((stamp,image))
                except Exception as e:self.q.put(('error','Window capture: '+str(e)))
            control=CaptureProcess()
            self.q.put(('source','Source: Star Citizen window • background capture'))
            while not self.stop.is_set():
                try:
                    frame=control.read()
                    if time.monotonic()-frame.stamp>3:continue
                    on_frame_arrived(frame,None)
                    began,image=frames.get(timeout=.5)
                except queue.Empty:
                    if closed.is_set():raise RuntimeError('Game capture closed. Restore the game window and restart capture.')
                    self.q.put(('error','Waiting for a new game frame (window may be minimized)'));continue
                try:
                    p=read_coordinates(image)
                    if time.monotonic()-began>MAX_SAMPLE_AGE:raise ValueError('OCR took too long; waiting for a newer frame')
                    good_count+=1
                    result=average.add(began,p)
                    self.q.put(('motion',began,p))
                    if result:self.q.put(('position',*result))
                    else:self.q.put(('warming',len(average.frames)))
                except Exception as e:
                    bad_count+=1
                    self.q.put(('error',str(e)))
                    (DATA/'ocr-diagnostic.json').write_text(json.dumps({'utc':time.time(),'ocr':OCR_LAST, 'error':str(e)},indent=2))
                if UNIT_READER is not None and UNIT_READER.latest_orientation is not None:
                    self.q.put(('orientation',began,UNIT_READER.latest_orientation))
                if UNIT_READER is not None and time.monotonic()>=next_shard_read:
                    try:shard=UNIT_READER.read_shard(image)
                    except Exception:shard=None
                    if time.monotonic()-began<=MAX_SAMPLE_AGE:self.q.put(('shard',began,shard))
                    next_shard_read=time.monotonic()+(15 if shard and shard==previous_shard else 5)
                    previous_shard=shard
                if UNIT_READER is not None and time.monotonic()>=next_system_read:
                    try:system=UNIT_READER.read_system(image)
                    except Exception:system=None
                    if system and time.monotonic()-began<=MAX_SAMPLE_AGE:self.q.put(('system',system))
                    next_system_read=time.monotonic()+45
                health={'utc':time.time(),'good':good_count,'rejected':bad_count,'frame_age_seconds':round(time.monotonic()-began,3),'averaging_frames':len(average.frames),'camdir':getattr(UNIT_READER,'latest_orientation',None),'rows':getattr(UNIT_READER,'evidence',[]),'method':OCR_LAST.get('method'),'field_reads':OCR_LAST.get('isolated_y'),'glyph_y':OCR_LAST.get('glyph_y'),'shard_rows':getattr(UNIT_READER,'shard_evidence',[])}
                health_tmp=DATA/'ocr-health.tmp';health_tmp.write_text(json.dumps(health,indent=2));health_tmp.replace(DATA/'ocr-health.json')
                self.stop.wait(max(0,1.0-(time.monotonic()-began)))
        except Exception as e:self.q.put(('fatal',str(e)))
        finally:
            if control is not None:
                try:control.stop()
                except Exception:pass
            self.q.put(('stopped',))
    def poll(self):
        self.ui_heartbeat=time.monotonic()
        deadline=time.monotonic()+.02
        try:
            for _ in range(40):
                if time.monotonic()>deadline:break
                item=self.q.get_nowait()
                if item[0]=='position':self.accept(item[1],item[2],item[3])
                elif item[0]=='asteroid':
                    self.asteroid_busy=False
                    self.root.after(0,lambda values=item[1:]:self.review_asteroid(*values))
                elif item[0]=='asteroid_error':
                    self.asteroid_busy=False
                    messagebox.showerror('Bookmark asteroid',item[1],parent=self.root)
                elif item[0]=='motion':
                    was_qt=self.motion.quantum;self.motion.add(item[1],item[2])
                    if self.motion.quantum and not was_qt:self.alerted=False;self.qt_seen_ahead=False
                elif item[0]=='contacts':
                    if self.running and self.contact_enabled.is_set():
                        self.contact_overlay.update(item[1],item[2],item[3],item[4])
                        self.scan_label_status.set(f'RS labels: {len(item[3])} contact(s) in last snapshot; fast badge checks; periodic discovery.')
                elif item[0]=='contact_error':self.scan_label_status.set(item[1])
                elif item[0]=='orientation':self.camdir_time=item[1];self.camdir=item[2]
                elif item[0]=='system':
                    if self.auto_system.get() and self.system_name.get()!=item[1]:self.system_name.set(item[1])
                elif item[0]=='shard':
                    stamp,shard=item[1:]
                    if shard and shard==self.shard_candidate and stamp-self.shard_candidate_time<=45:
                        self.shard_id=shard;self.shard_time=stamp
                    elif shard:
                        self.shard_id=None
                    if shard:self.shard_candidate=shard;self.shard_candidate_time=stamp
                elif item[0]=='warming':self.status.set(f'Collecting readable frames {item[1]}/8…')
                elif item[0]=='error':self.status.set('Reading rejected: '+item[1][:130])
                elif item[0]=='stopped':
                    self.running=False;self.scan_label_status.set('RS labels: capture stopped. Click Start capture.')
                elif item[0]=='fatal':self.status.set('Capture failed: '+item[1])
                elif item[0]=='source':self.source_text.set(item[1])
                elif item[0]=='preview':
                    self.preview_image=item[1]
                    if self.preview_window is not None and self.preview_window.winfo_exists():self.show_preview()
        except queue.Empty:pass
        self.contact_overlay.render(self.overlay.game_rect(),self.camdir,self.running and self.contact_enabled.is_set(),time.monotonic())
        if self.position is not None:
            age=time.monotonic()-self.last_good
            self.reading_age.set(f'Last successful reading: {age:.1f}s ago'+(' — check capture preview or minimized game' if age>FRESH_SECONDS else ''))
        if self.pending_bookmark is not None:
            remaining=self.pending_bookmark-time.monotonic()
            self.countdown_text.set(f'Return to the game. Saving in {math.ceil(remaining)} seconds…' if remaining>0 else 'Waiting for a fresh readable game frame…')
            if remaining < -15:
                self.pending_bookmark=None;self.pending_name=None
                self.countdown_text.set('Bookmark not saved: no readable game frame after countdown.')
        now=time.monotonic()
        self.shard_text.set('Current shard: '+(self.current_shard() or 'unknown / checking'))
        fresh=self.running and self.position is not None and now-self.last_good<=FRESH_SECONDS
        motion_fresh=self.running and self.motion.fresh(now)
        quantum=self.motion.quantum
        distance=None;held=False;travel_lines=[]
        orientation_fresh=self.camdir is not None and now-self.camdir_time<=FRESH_SECONDS
        if self.target and self.position is not None:
            velocity=self.motion.velocity
            estimated,held=estimate_position(self.position,self.position_time,velocity,now)
            if motion_fresh:estimated=self.motion.position(now)
            distance,closing,eta,miss=approach(estimated,self.target['xyz_km'],velocity)
            label='ESTIMATE HELD' if held and not motion_fresh else ('Distance' if motion_fresh else 'Estimated distance')
            if quantum:
                if motion_fresh and eta is not None:
                    try:lead=max(0,min(120,float(self.lead.get())))
                    except ValueError:lead=3
                    if eta>lead:self.qt_seen_ahead=True
                    if self.armed and not self.alerted and (self.qt_seen_ahead or eta>=0) and eta<=lead:
                        self.alerted=True
                        threading.Thread(target=lambda:winsound.Beep(1200,600),daemon=True).start()
                    travel_lines=[('CLOSEST APPROACH IN '+duration_text(eta)) if eta>=0 else ('PASSED CLOSEST APPROACH '+duration_text(-eta)+' AGO'),
                                  f'Projected miss: {miss:,.2f} km',
                                  ('DROP OUT NOW — alert fired' if self.alerted else
                                   ('Exit cue in '+duration_text(eta-lead) if eta>lead else 'DROP OUT NOW') if self.armed else 'Audio cue OFF — arm in app'),
                                  'Manual QT exit • prediction at current velocity']
                else:travel_lines=['QT countdown paused','Waiting for fresh coordinates',
                                   'Audio cue armed' if self.armed else 'Audio cue off']
            else:
                travel_lines=[arrival_text(distance,closing,norm(velocity),motion_fresh)]
            self.nav_text.set(self.target['name']+'\n'+f'{label}: {distance:,.3f} km'+'\n'+'\n'.join(travel_lines))
            if orientation_fresh and not quantum:self.course.update_angles(self.camdir,sub(self.target['xyz_km'],estimated))
            else:self.course.reset()
        self.overlay.render(self.target['name'] if self.target else None,distance,self.course,
                            fresh or motion_fresh,self.overlay_enabled.get(),self.motion_angle,held and not motion_fresh,
                            orientation_fresh,quantum,travel_lines)
        self.root.after(250,self.poll)
    def accept(self,t,p,newest=None):
        newest=t if newest is None else newest
        if time.monotonic()-newest>MAX_SAMPLE_AGE:
            self.status.set('Screenshot processing too slow — sample rejected');return
        if self.samples and t-self.samples[-1][0]>FRESH_SECONDS:self.samples.clear()
        self.position=p;self.last_good=newest;self.position_time=t;self.samples.append((t,p))
        self.pos_text.set('Root XYZ km:\n'+ '  '.join(f'{v:,.4f}' for v in p))
        self.status.set(f'8-frame average • sample age {time.monotonic()-t:.2f}s')
        if self.pending_bookmark is not None and t>=self.pending_bookmark:
            name=self.pending_name;self.pending_bookmark=None;self.pending_name=None
            self.bookmark(quiet=True,name_override=name)
            self.countdown_text.set('Timed bookmark saved. Confirmation beep played.')
        if t-self.last_logged>=.5:
            path=DATA/'travels.csv';exists=path.exists()
            with path.open('a',newline='') as f:
                writer=csv.writer(f)
                if not exists:writer.writerow(['utc','monotonic_seconds','x_km','y_km','z_km'])
                writer.writerow([time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime(time.time()-(time.monotonic()-t))),t,*p])
            self.last_logged=t
            diagnostic={'utc':time.time(),'sample_age_seconds':time.monotonic()-t,'xyz_km':p,'source':'Star Citizen window / Windows Graphics Capture','target':self.target['name'] if self.target else None,'overlay_visible':self.overlay.visible,'guidance_phase':self.course.phase,'guidance_prompt':self.course.heading,'camdir':self.camdir,'aim_yaw_degrees':self.course.yaw,'aim_pitch_degrees':self.course.pitch,'shard_id':self.current_shard()}
            status_tmp=DATA/'live-status.tmp';status_tmp.write_text(json.dumps(diagnostic,indent=2));status_tmp.replace(DATA/'live-status.json')
        if not self.target:return
        delta=sub(self.target['xyz_km'],p)
        course_v=fit_velocity(list(self.samples)[-5:])
        self.motion_angle=angle_between(delta,course_v) if norm(delta)>.0001 and norm(course_v)>.02 else None
        if 'forward' in self.basis and 'right' in self.basis:
            yaw,pitch=steering(sub(self.target['xyz_km'],p),self.basis['forward'],self.basis['right'])
            self.steer_text.set(f'From calibrated attitude: {abs(yaw):.1f}° '+('RIGHT' if yaw>=0 else 'LEFT')+f', {abs(pitch):.1f}° '+('UP' if pitch>=0 else 'DOWN'))
    def bookmark(self,cached=False,quiet=False,name_override=None):
        try:
            if cached:
                if self.position is None:raise ValueError('No coordinate reading has been captured yet.')
                p=self.position
            else:p=self.fresh()
        except ValueError as e:return messagebox.showerror('Bookmark',str(e))
        name=name_override or self.name.get().strip()
        if not name:return
        age=time.monotonic()-self.position_time
        from bookmark_links import validate
        try:
            b=validate({'name':name,'system':self.system_name.get(),'xyz_km':list(p),'kind':'space bookmark',
                        'shard_id':self.current_shard() if age<=45 else None,'category':self.new_category.get(),
                        'utc':time.strftime('%Y-%m-%dT%H:%M:%SZ',time.gmtime(time.time()-age)),
                        'reading_age_at_save_seconds':round(age,2),'saved_from_cached_reading':cached})
            self.add_bookmarks([b])
        except (ValueError,OSError) as e:return messagebox.showerror('Save bookmark',str(e),parent=self.root)
        self.status.set(f'Saved {name} (reading {age:.1f}s old)')
        if quiet:threading.Thread(target=lambda:winsound.Beep(850,150),daemon=True).start()
    def select(self):
        selected=self.list.curselection()
        if selected:
            target=self.visible_bookmarks[selected[0]]
            from navigator_ui import bookmark_summary
            self.bookmark_details.set(bookmark_summary(target))
            if target is self.target:return
            self.target=target;self.armed=False;self.alerted=False;self.qt_seen_ahead=False;self.clear_cal();self.course.reset();self.motion_angle=None
    def rename_bookmark(self):
        selected=self.list.curselection()
        if not selected:return messagebox.showinfo('Rename bookmark','Select a bookmark first.',parent=self.root)
        index=selected[0];bookmark=self.visible_bookmarks[index]
        name=simpledialog.askstring('Rename bookmark','Bookmark name:',initialvalue=bookmark['name'],parent=self.root)
        if name is None:return
        name=name.strip()
        if not name:return messagebox.showinfo('Rename bookmark','Enter a name.',parent=self.root)
        old_name=bookmark['name']
        if name==old_name:return
        bookmark['name']=name
        try:self.save()
        except OSError as e:
            bookmark['name']=old_name
            return messagebox.showerror('Rename bookmark','Could not save the new name: '+str(e),parent=self.root)
        self.refresh_bookmarks(bookmark)
        self.status.set('Renamed bookmark to '+name)

    def delete_bookmark(self):
        selected=self.list.curselection()
        if not selected:return messagebox.showinfo('Delete bookmark','Select a bookmark first.',parent=self.root)
        bookmark=self.visible_bookmarks[selected[0]]
        if not messagebox.askyesno('Delete bookmark',f'Delete "{bookmark["name"]}"?',parent=self.root):return
        index=next(i for i,b in enumerate(self.bookmarks) if b is bookmark)
        self.bookmarks.pop(index)
        try:self.save()
        except OSError as e:
            self.bookmarks.insert(index,bookmark)
            return messagebox.showerror('Delete bookmark','Could not delete bookmark: '+str(e),parent=self.root)
        if self.target is bookmark:self.clear_target()
        self.list.selection_clear(0,'end')
        self.refresh_bookmarks()
        self.bookmark_details.set('')
        self.status.set('Deleted bookmark: '+bookmark['name'])

    def clear_target(self):
        self.target=None;self.armed=False;self.alerted=False;self.list.selection_clear(0,'end')
        self.nav_text.set('Choose a bookmark to navigate.');self.course.reset();self.overlay.hide()
    def arm(self):
        if not self.target:return messagebox.showinfo('Alert','Choose a navigation bookmark first.')
        try:
            lead=float(self.lead.get())
            if not math.isfinite(lead) or not 0<=lead<=120:raise ValueError('Lead must be between 0 and 120 seconds.')
        except ValueError as e:return messagebox.showerror('Alert',str(e))
        self.armed=not self.armed;self.alerted=False
        self.status.set('QT alert armed — waiting for live coordinates' if self.armed else 'QT alert disarmed')
    def begin_cal(self,axis):
        try:self.cal_start=(axis,self.fresh())
        except ValueError as e:return messagebox.showerror('Calibration',str(e))
        self.status.set('Move '+axis+' without rotating, stop, then Finish movement.')
    def finish_cal(self):
        try:
            if not self.cal_start:raise ValueError('Start Forward or Right first.')
            axis,p=self.cal_start;delta=sub(self.fresh(),p)
            if norm(delta)<.02:raise ValueError('Move at least 20 metres for a useful calibration.')
            candidate=unit(delta)
            other=self.basis.get('right' if axis=='forward' else 'forward')
            if other and abs(dot(candidate,other))>.3:raise ValueError('Movements are not perpendicular. Clear and retry without rotating.')
            self.basis[axis]=candidate;self.cal_start=None;self.status.set(axis.title()+' calibrated')
        except ValueError as e:messagebox.showerror('Calibration',str(e))
    def clear_cal(self):self.basis={};self.cal_start=None;self.steer_text.set('Direction not calibrated at this attitude.')
    def close(self):self.stop.set();self.root.destroy()

if __name__=='__main__':
    import argparse
    parser=argparse.ArgumentParser();parser.add_argument('--capture',action='store_true');parser.add_argument('--name');parser.add_argument('--target-last',action='store_true');parser.add_argument('--arm-qt',action='store_true')
    parser.add_argument('--target-name')
    parser.add_argument('--no-capture',action='store_true')
    args=parser.parse_args()
    from scbm_protocol import acquire_instance,poll as poll_links
    instance_handle=acquire_instance()
    if instance_handle is None:raise SystemExit(0)
    # Yield CPU scheduling priority to the game.
    kernel=ctypes.WinDLL('kernel32',use_last_error=True)
    kernel.GetCurrentProcess.restype=wintypes.HANDLE
    kernel.SetPriorityClass.argtypes=[wintypes.HANDLE,wintypes.DWORD]
    kernel.SetPriorityClass(kernel.GetCurrentProcess(),0x4000)
    import faulthandler
    DATA.mkdir(exist_ok=True)
    trace_file=(DATA/'thread-diagnostic.log').open('w')
    faulthandler.enable(file=trace_file)
    faulthandler.dump_traceback_later(10, file=trace_file)
    root=tk.Tk();app=Navigator(root,auto_capture=not args.no_capture)
    faulthandler.cancel_dump_traceback_later()
    app.ui_heartbeat=time.monotonic()
    def watch_ui():
        reported=False
        while not app.stop.wait(3):
            stalled=time.monotonic()-app.ui_heartbeat>10
            if stalled and not reported:
                faulthandler.dump_traceback(file=trace_file);trace_file.flush()
            reported=stalled
    threading.Thread(target=watch_ui,daemon=True).start()
    if args.name:app.name.set(args.name)
    if args.target_last and app.bookmarks:
        app.focus_bookmark(app.bookmarks[-1])
    if args.target_name:
        for index,bookmark in enumerate(app.bookmarks):
            if bookmark['name']==args.target_name:
                app.focus_bookmark(bookmark);break
    if args.arm_qt and app.target:app.arm()
    if args.capture:root.after(500,app.start)
    root.after(500,lambda:poll_links(app))
    faulthandler.dump_traceback_later(15,file=trace_file)
    root.after(2000,faulthandler.cancel_dump_traceback_later)
    root.mainloop()
