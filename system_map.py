"""Event-driven orthographic system map. Tk canvas only; no render loop or GPU dependencies."""
import json
import math
import time
import random
from PIL import Image, ImageDraw, ImageTk
from pathlib import Path
import tkinter as tk
from tkinter import ttk
from quantum_routes import suggest

BG='#09131d'; GRID='#132731'; CYAN='#68dfeb'; AMBER='#efbd70'; WHITE='#e2edf2'; MUTED='#92aab8'


def project(p, center, yaw, tilt, scale, width, height):
    x,y,z=(p[i]-center[i] for i in range(3))
    u=x*math.cos(yaw)-y*math.sin(yaw)
    v=x*math.sin(yaw)+y*math.cos(yaw)
    return width/2+u*scale,height/2+(v*math.sin(tilt)-z*math.cos(tilt))*scale


class SystemMap(ttk.Frame):
    def __init__(self,parent,app):
        super().__init__(parent,padding=8)
        self.app=app;self.system=tk.StringVar(value=app.system_name.get());self.mode='system'
        self.yaw=-.25;self.tilt=.65;self.center=(0,0,0);self.scale=1;self.pending=None
        self.signature=None;self.hit=[];self.last_drag=None;self.closed=False;self.first_size=True
        self.selected_marker=None;self.background=None;self.background_size=None;self.star_glow=None
        self.show_orbits=tk.BooleanVar(value=True);self.show_halo=tk.BooleanVar(value=True);self.band=tk.StringVar(value='All bands');self.layer_key=None;self.layer_image=None
        self.halo=json.loads((Path(__file__).with_name('route-data')/'aaron-halo.json').read_text())
        self.markers=json.loads((Path(__file__).with_name('route-data')/'quantum-markers.json').read_text())['markers']
        try:self.stars=json.loads((Path(__file__).with_name('route-data')/'stars.json').read_text())
        except (OSError,ValueError):self.stars={}
        bar=ttk.Frame(self);bar.pack(fill='x')
        for system in ('Stanton','Pyro','Nyx'):
            ttk.Radiobutton(bar,text=system,variable=self.system,value=system,command=self.change_system).pack(side='left',padx=5)
        for label,command in [('System',self.fit_system),('Fit route',self.fit_route),('Top',lambda:self.view(math.pi/2)),('3D',lambda:self.view(.65)),('Side',lambda:self.view(.001)),('Focus star',self.focus_star)]:
            ttk.Button(bar,text=label,command=command).pack(side='left',padx=2)
        layers=ttk.Frame(self);layers.pack(fill='x',pady=(6,0))
        ttk.Checkbutton(layers,text='Planet rings',variable=self.show_orbits,command=self.request_draw).pack(side='left',padx=5)
        ttk.Checkbutton(layers,text='Aaron Halo',variable=self.show_halo,command=self.request_draw).pack(side='left',padx=5)
        bands=ttk.Combobox(layers,textvariable=self.band,values=['All bands']+['Band '+str(i) for i in range(1,11)],state='readonly',width=12);bands.pack(side='left',padx=5);bands.bind('<<ComboboxSelected>>',lambda e:self.request_draw())
        ttk.Label(layers,text='Halo boundaries approximate · community survey',foreground=MUTED).pack(side='left',padx=8)
        body=ttk.Frame(self);body.pack(fill='both',expand=True,pady=8)
        side=ttk.Frame(body,width=200);side.pack(side='left',fill='y',padx=(0,8))
        ttk.Label(side,text='BOOKMARKS',foreground=CYAN).pack(anchor='w')
        self.booklist=tk.Listbox(side,width=23,height=14,bg=BG,fg=WHITE,selectbackground='#24556a',exportselection=False)
        self.booklist.pack(fill='both',expand=True);self.booklist.bind('<<ListboxSelect>>',self.pick_bookmark)
        for label,command in [('Edit selected',app.edit_bookmark),('Copy link',app.copy_bookmarks),('Bookmark here',app.bookmark),('Bookmark asteroid',app.bookmark_asteroid),('Enter coordinates',app.manual_bookmark)]:
            ttk.Button(side,text=label,command=command).pack(fill='x',pady=2)
        ttk.Label(side,text='Drag: orbit\nShift-drag: pan\nWheel: zoom\nClick QT point: show name\nClick bookmark: select\nDouble-click: edit',foreground=MUTED).pack(anchor='w',pady=8)
        main=ttk.Frame(body);main.pack(side='left',fill='both',expand=True)
        self.info=tk.StringVar();ttk.Label(main,textvariable=self.info,foreground=CYAN,wraplength=650).pack(anchor='w')
        self.canvas=tk.Canvas(main,bg=BG,height=390,highlightthickness=0);self.canvas.pack(fill='both',expand=True)
        self.profile=tk.Canvas(main,bg=BG,height=120,highlightthickness=0);self.profile.pack(fill='x',pady=(5,0))
        ttk.Label(main,text='△ You    ◇ Destination    □ QT point    ☀ Star (no QT)   •   Star positions approximate',foreground=MUTED).pack(anchor='w')
        self.canvas.bind('<Configure>',self.resized)
        self.canvas.bind('<ButtonPress-1>',self.press);self.canvas.bind('<B1-Motion>',self.drag)
        self.canvas.bind('<ButtonRelease-1>',self.release);self.canvas.bind('<Double-Button-1>',self.double_click)
        self.canvas.bind('<Motion>',self.hover)
        self.canvas.bind('<MouseWheel>',self.zoom)
        self.bind('<Destroy>',self.destroyed,add='+')
        self.fit_system();self.after(500,self.tick)

    def destroyed(self,event):
        if event.widget is self:self.closed=True

    def resized(self,event):
        if self.first_size and event.width>100:
            self.first_size=False;self.fit_system()
        self.request_draw()

    def current_markers(self):return [m for m in self.markers if m['system']==self.system.get()]
    def bookmarks(self):return [b for b in self.app.bookmarks if b.get('system','').casefold()==self.system.get().casefold()]
    def target(self):
        b=self.app.target
        return b if b and b.get('system','').casefold()==self.system.get().casefold() else None
    def ship(self):return self.app.position if self.app.system_name.get().casefold()==self.system.get().casefold() else None
    def route(self):
        b=self.target()
        return suggest(b,self.markers) if b else None
    def change_system(self):self.selected_marker=None;self.mode='system';self.fit_system()
    def view(self,tilt):self.tilt=tilt;self.request_draw()
    def fit(self,points):
        if not points:return
        self.center=tuple((min(p[i] for p in points)+max(p[i] for p in points))/2 for i in range(3))
        w=max(400,self.canvas.winfo_width());h=max(300,self.canvas.winfo_height())
        projected=[project(p,self.center,self.yaw,self.tilt,1,w,h) for p in points]
        du=(min(p[0] for p in projected)+max(p[0] for p in projected))/2-w/2
        dv=(min(p[1] for p in projected)+max(p[1] for p in projected))/2-h/2
        self.center=tuple(self.center[i]+du*(math.cos(self.yaw),-math.sin(self.yaw),0)[i]+dv*(math.sin(self.yaw)*math.sin(self.tilt),math.cos(self.yaw)*math.sin(self.tilt),-math.cos(self.tilt))[i] for i in range(3))
        self.scale=min((w-160)/max(1,max(p[0] for p in projected)-min(p[0] for p in projected)),(h-130)/max(1,max(p[1] for p in projected)-min(p[1] for p in projected)))
        self.request_draw()
    def fit_system(self):
        self.mode='system';points=[m['xyz_km'] for m in self.current_markers()]+[b['xyz_km'] for b in self.bookmarks()]
        star=self.stars.get(self.system.get())
        if star:points.append(star['xyz_km'])
        self.fit(points)
    def fit_route(self):
        self.mode='route';b=self.target();r=self.route()
        if not b:return self.fit_system()
        points=[b['xyz_km']]
        if self.ship():points.append(self.ship())
        if r:
            points += [r['celestial']['xyz_km']] if 'celestial' in r else [r['a']['xyz_km'],r['b']['xyz_km']]
        self.fit(points)
    def focus_star(self):
        star=self.stars.get(self.system.get())
        if star:self.center=tuple(star['xyz_km']);self.request_draw()
    def request_draw(self):
        if self.pending is None and not self.closed:self.pending=self.after(33,self.draw)
    def pick_bookmark(self,event=None):
        choice=self.booklist.curselection()
        if choice:self.activate(self.bookmarks()[choice[0]])
    def activate(self,b):
        self.app.category_filter.set('All categories');self.app.ore_filter.set('Any ore');self.app.applied_ore='';self.app.applied_quality=None
        self.app.hide_other_systems.set(False);self.app.focus_bookmark(b);self.fit_route()
    def press(self,e):self.last_drag=(e.x,e.y);self.moved=False
    def drag(self,e):
        if self.last_drag is None:return
        dx,dy=e.x-self.last_drag[0],e.y-self.last_drag[1];self.last_drag=(e.x,e.y);self.moved=True
        if e.state&1:
            c=math.cos(self.yaw);s=math.sin(self.yaw);v=dy/max(.15,math.sin(self.tilt))
            self.center=(self.center[0]-(dx*c+v*s)/self.scale,self.center[1]-(-dx*s+v*c)/self.scale,self.center[2])
        else:self.yaw+=dx*.008;self.tilt=max(.001,min(math.pi/2,self.tilt+dy*.006))
        self.request_draw()
    def release(self,e):
        if not self.moved:
            hits=sorted(((math.hypot(e.x-x,e.y-y),kind,obj) for x,y,kind,obj in self.hit if math.hypot(e.x-x,e.y-y)<14),key=lambda item:item[0])
            if hits:
                _,kind,obj=hits[0]
                if kind=='bookmark':self.activate(obj)
                else:self.selected_marker=obj;self.request_draw()
            else:self.selected_marker=None;self.request_draw()
        self.last_drag=None
    def double_click(self,e):
        if any(kind=='bookmark' and math.hypot(e.x-x,e.y-y)<14 for x,y,kind,obj in self.hit):self.app.edit_bookmark()
    def hover(self,e):
        self.canvas.configure(cursor='hand2' if any(math.hypot(e.x-x,e.y-y)<14 for x,y,kind,obj in self.hit) else '')
    def backdrop(self,w,h):
        if self.background_size!=(w,h):
            # Static, cached raster: no particle simulation or per-frame image work.
            im=Image.new('RGB',(w,h));d=ImageDraw.Draw(im)
            for y in range(h):
                t=y/max(1,h-1);d.line((0,y,w,y),fill=(5+int(4*t),12+int(9*t),21+int(12*t)))
            rng=random.Random(7103)
            for _ in range(min(420,max(100,w*h//2700))):
                x=rng.randrange(w);y=rng.randrange(h);bright=rng.random()
                color='#7392a8' if bright<.85 else '#e2edf4'
                d.point((x,y),fill=color)
                if bright>.97:
                    d.line((x-2,y,x+2,y),fill='#294655');d.line((x,y-2,x,y+2),fill='#294655');d.point((x,y),fill='#dceef4')
            self.background=ImageTk.PhotoImage(im,master=self.canvas);self.background_size=(w,h)
        self.canvas.create_image(0,0,image=self.background,anchor='nw')
    def zoom(self,e):self.scale=max(1e-10,min(50,self.scale*(1.15 if e.delta>0 else 1/1.15)));self.request_draw();return 'break'
    def tick(self):
        if self.closed:return
        # A lightweight 2 Hz state check. Hidden map performs no rendering.
        if self.winfo_viewable():
            signature=repr((self.app.position,self.app.target,self.app.bookmarks,self.app.system_name.get(),bool(self.app.position and time.monotonic()-self.app.last_good<3)))
            if signature!=self.signature:
                self.signature=signature;self.request_draw()
        self.after(500,self.tick)
    def draw(self):
        self.pending=None;c=self.canvas;c.delete('all');self.hit=[]
        w,h=c.winfo_width(),c.winfo_height()
        if w<10:return
        self.backdrop(w,h)
        at=lambda p:project(p,self.center,self.yaw,self.tilt,self.scale,w,h)
        # Fixed-count grid, independent of zoom and system size.
        spacing=10**math.floor(math.log10(max(1, w/self.scale/8)))
        cx,cy,cz=self.center
        for i in range(-8,9):
            for a,b in [((cx+i*spacing,cy-8*spacing,cz),(cx+i*spacing,cy+8*spacing,cz)),((cx-8*spacing,cy+i*spacing,cz),(cx+8*spacing,cy+i*spacing,cz))]:c.create_line(*at(a),*at(b),fill=GRID)
        self.draw_environment(w,h,at)
        def icon(p,label,color,kind='square',size=4):
            x,y=at(p)
            if not (-50<x<w+50 and -30<y<h+30):return x,y
            if label and kind!='star':
                gx,gy=at((p[0],p[1],cz))
                if abs(y-gy)>8:c.create_line(x,y,gx,gy,fill='#34505d',dash=(2,4));c.create_oval(gx-5,gy-2,gx+5,gy+2,outline='#34505d')
            if kind=='diamond':
                c.create_oval(x-17,y-8,x+17,y+8,outline='#68522d')
                c.create_polygon(x,y-8,x+8,y,x,y+8,x-8,y,outline=color,fill='#574221',width=2)
            elif kind=='ship':c.create_polygon(x,y-8,x+7,y+6,x,y+2,x-7,y+6,outline=color,fill=BG,width=2)
            elif kind=='star':
                if self.star_glow is None:
                    glow=Image.new('RGBA',(81,81))
                    for gy in range(81):
                        for gx in range(81):
                            distance=math.hypot(gx-40,gy-40)
                            glow.putpixel((gx,gy),(255,209,112,int(85*math.exp(-distance*distance/270))))
                    self.star_glow=ImageTk.PhotoImage(glow,master=c)
                c.create_image(x,y,image=self.star_glow)
                c.create_oval(x-7,y-7,x+7,y+7,fill=color,outline=color)
                for a in range(0,360,45):
                    angle=math.radians(a);c.create_line(x+10*math.cos(angle),y+10*math.sin(angle),x+14*math.cos(angle),y+14*math.sin(angle),fill=color)
            elif kind=='planet':
                c.create_oval(x-8,y-8,x+8,y+8,fill='#243e50',outline='#7494a8');c.create_oval(x-5,y-6,x+2,y+1,fill='#405d70',outline='')
            else:
                c.create_rectangle(x-size-2,y-size-2,x+size+2,y+size+2,outline='#294553')
                c.create_rectangle(x-size,y-size,x+size,y+size,outline=color,width=2)
            if label:
                anchor='e' if x>w-210 else 'w';lx=x-14 if anchor=='e' else x+14
                text=c.create_text(lx,y-13,text=label,fill=color,anchor=anchor,font=('Segoe UI Semibold',10))
                bounds=c.bbox(text);panel=c.create_rectangle(bounds[0]-4,bounds[1]-3,bounds[2]+4,bounds[3]+3,fill='#0a1924',outline='');c.tag_lower(panel,text)
            return x,y
        r=self.route();b=self.target();important=set()
        if r:
            if 'celestial' in r:important.add(r['celestial']['name'])
            else:
                important={r['a']['name'],r['b']['name']}
                c.create_line(*at(r['a']['xyz_km']),*at(r['b']['xyz_km']),fill='#174657',width=6)
                c.create_line(*at(r['a']['xyz_km']),*at(r['b']['xyz_km']),fill=CYAN,width=2)
                c.create_line(*at(r['position']),*at(b['xyz_km']),fill=AMBER,dash=(5,4))
                x,y=at(r['position']);c.create_oval(x-5,y-5,x+5,y+5,outline=CYAN,width=2)
        occupied=[]
        for m in sorted(self.current_markers(),key=lambda m:(m['name'] not in important,-m.get('body_radius_km',0))):
            x,y=at(m['xyz_km']);label=m['name'] if m['name'] in important or m.get('body_radius_km',0)>0 else ''
            if label and m['name'] not in important and any(abs(x-u)<145 and abs(y-v)<22 for u,v in occupied):label=''
            if label:occupied.append((x,y))
            icon(m['xyz_km'],label,WHITE if m['name'] in important else MUTED,'planet' if m.get('body_radius_km',0)>0 else 'square')
            if 0<=x<=w and 0<=y<=h:self.hit.append((x,y,'marker',m))
        for mark in self.bookmarks():
            selected=mark is b;x,y=icon(mark['xyz_km'],mark['name'] if selected or self.mode=='system' else '',AMBER if selected else '#a88d63','diamond');self.hit.append((x,y,'bookmark',mark))
        star=self.stars.get(self.system.get())
        if star:
            x,y=icon(star['xyz_km'],self.system.get()+' star · No QT','#f2dfaf','star')
            if not (15<x<w-15 and 15<y<h-15):c.create_text(15,18,text='☀ '+self.system.get()+' star '+('←' if x<w/2 else '→')+' · off-screen / no QT',fill='#f2dfaf',anchor='w')
        ship=self.ship();fresh=ship is not None and time.monotonic()-self.app.last_good<3
        if ship:icon(ship,'YOU' if fresh else 'LAST KNOWN POSITION',CYAN if fresh else MUTED,'ship')
        if self.selected_marker:
            m=self.selected_marker;x,y=at(m['xyz_km'])
            if 0<=x<=w and 0<=y<=h:
                c.create_oval(x-13,y-13,x+13,y+13,outline=CYAN,width=2)
                icon(m['xyz_km'],m['name']+' · QT point',CYAN,'planet' if m.get('body_radius_km',0)>0 else 'square')
        c.create_rectangle(1,1,w-2,h-2,outline='#29404e')
        self.info.set(self.system.get().upper()+' / '+self.mode.upper()+'   •   '+('Live position' if fresh else 'Position unavailable / stale')+'   •   '+(b['name'] if b else 'Select a bookmark'))
        selected=self.booklist.curselection();names=[mark['name'] for mark in self.bookmarks()]
        if list(self.booklist.get(0,'end'))!=names:
            self.booklist.delete(0,'end')
            for name in names:self.booklist.insert('end',name)
        if b:
            for i,mark in enumerate(self.bookmarks()):
                if mark is b:self.booklist.selection_clear(0,'end');self.booklist.selection_set(i)
        self.draw_profile(r,b,ship)
    def draw_environment(self,w,h,at):
        # Cache raster geometry separately from moving ship/route markers.
        key=(w,h,self.system.get(),self.center,self.yaw,self.tilt,self.scale,self.show_orbits.get(),self.show_halo.get(),self.band.get(),self.selected_marker and self.selected_marker['name'])
        if key!=self.layer_key:
            im=Image.new('RGBA',(w,h));d=ImageDraw.Draw(im)
            def ring(radius,color,width=1):
                points=[at((radius*math.cos(i*math.tau/256),radius*math.sin(i*math.tau/256),0)) for i in range(257)]
                d.line(points,fill=color,width=width)
            primary={'Stanton':{'Hurston','Crusader','ArcCorp','microTech'},'Pyro':{'Pyro I','Monox','Bloom','Pyro IV','Pyro V','Terminus'},'Nyx':set()}
            if self.show_orbits.get():
                for m in self.current_markers():
                    if m['name'] in primary.get(self.system.get(),set()):
                        ring(math.hypot(*m['xyz_km'][:2]),'#5bc5d0' if m is self.selected_marker else '#26505e',2 if m is self.selected_marker else 1)
            if self.system.get()=='Stanton' and self.show_halo.get():
                rng=random.Random(31415)
                for b in self.halo['bands']:
                    selected=self.band.get()=='Band '+str(b['number'])
                    count=440 if b['number']==5 else 240
                    for _ in range(count):
                        radius=math.sqrt(rng.uniform(b['inner_km']**2,b['outer_km']**2));angle=rng.uniform(0,math.tau)
                        x,y=at((radius*math.cos(angle),radius*math.sin(angle),rng.uniform(-2000,2000)))
                        if 0<=x<w and 0<=y<h:
                            color=rng.choice(['#67706b','#59665f','#899083']) if not selected else '#d8b575'
                            d.line((x,y,x+1,y+1),fill=color)
                    if selected or (b['outer_km']-b['inner_km'])*self.scale>4:
                        ring(b['inner_km'],'#c6a569' if selected else '#3d4b47');ring(b['outer_km'],'#c6a569' if selected else '#3d4b47')
                ring(self.halo['bands'][0]['inner_km'],'#46534c');ring(self.halo['bands'][-1]['outer_km'],'#46534c')
            self.layer_image=ImageTk.PhotoImage(im,master=self.canvas);self.layer_key=key
        self.canvas.create_image(0,0,image=self.layer_image,anchor='nw')
        if self.system.get()=='Stanton' and self.show_halo.get():
            c=self.canvas
            detail=(self.halo['bands'][-1]['outer_km']-self.halo['bands'][0]['inner_km'])*self.scale>85
            for b in self.halo['bands']:
                selected=self.band.get()=='Band '+str(b['number'])
                if not (selected or detail or b['number']==5):continue
                angle=-self.yaw-math.pi/2+(b['number']-5)*.055
                radius=(b['inner_km']+b['outer_km'])/2
                x,y=at((radius*math.cos(angle),radius*math.sin(angle),0))
                if not (20<x<w-20 and 15<y<h-20):continue
                label=('AARON HALO' if not detail and not selected else 'B'+str(b['number']))
                if selected:label=f"AARON HALO · Band {b['number']} · {b['inner_km']/1e6:.3f}–{b['outer_km']/1e6:.3f} Gm"
                t=c.create_text(x,y-12,text=label,fill=AMBER if selected else '#a7b6a5',font=('Segoe UI',9),anchor='s')
                box=c.bbox(t);bg=c.create_rectangle(*box,fill=BG,outline='');c.tag_lower(bg,t)

    def draw_profile(self,r,b,ship):
        c=self.profile;c.delete('all');w=c.winfo_width()
        c.create_text(12,12,text='ROUTE PROFILE — offset from QT path (magnified independently)',fill=MUTED,anchor='w')
        if not r or 'celestial' in r:
            c.create_text(12,55,text='QT: '+r['celestial']['name'] if r else 'Select a destination to compare its route.',fill=CYAN,anchor='w');return
        a,z=r['a']['xyz_km'],r['b']['xyz_km'];length=math.dist(a,z)
        c.create_line(35,90,w-35,90,fill=CYAN)
        for x,label in [(35,r['a']['name']),(w-35,r['b']['name'])]:c.create_rectangle(x-4,86,x+4,94,outline=WHITE);c.create_text(x,108,text=label,fill=WHITE,anchor='w' if x==35 else 'e',font=('Segoe UI',8))
        x=35+r['fraction']*(w-70);c.create_line(x,90,x,40,fill=AMBER,dash=(4,3));c.create_polygon(x,33,x+7,40,x,47,x-7,40,outline=AMBER,fill=BG)
        c.create_text(x+12,40,text='Destination / offset',fill=AMBER,anchor='w')
        if ship:
            f=sum((ship[i]-a[i])*(z[i]-a[i]) for i in range(3))/length**2;px=35+max(0,min(1,f))*(w-70)
            c.create_polygon(px,82,px+6,96,px-6,96,fill=CYAN);c.create_text(px,72,text='YOU',fill=CYAN)
