"""Resolution-independent overlay placement and persistent named presets."""
import json
import math
from pathlib import Path
import tkinter as tk
from tkinter import ttk, messagebox, simpledialog

PATH = Path(__file__).resolve().parent / 'navigator-data' / 'overlay-settings.json'

def position(value):
    if not isinstance(value, (list, tuple)) or len(value) != 2:
        raise ValueError('Position needs horizontal and vertical percentages.')
    result = [float(v) for v in value]
    if not all(math.isfinite(v) and 0 <= v <= 100 for v in result):
        raise ValueError('Positions must be between 0 and 100.')
    return result

def placement(rect, value):
    """Percent of available travel, so even edge positions keep the whole overlay visible."""
    left, top, width, height = rect
    x, y = position(value)
    w, h = min(460, width), min(265, height)
    return round(left + (width-w)*x/100), round(top + (height-h)*y/100), w, h

class Settings:
    def __init__(self, path=PATH):
        self.path = path
        self.current = [50., 50.]
        self.presets = {}
        try:
            data = json.loads(path.read_text(encoding='utf-8'))
            current = position(data['position'])
            presets = {str(k): position(v) for k,v in data.get('presets', {}).items()}
            self.current, self.presets = current, presets
        except (OSError, ValueError, KeyError, TypeError, AttributeError):
            pass

    def save(self, current, presets):
        current = position(current)
        presets = {k: position(v) for k,v in presets.items()}
        self.path.parent.mkdir(parents=True, exist_ok=True)
        temp = self.path.with_suffix('.tmp')
        temp.write_text(json.dumps({'position':current, 'presets':presets}, indent=2), encoding='utf-8')
        temp.replace(self.path)
        self.current, self.presets = current, presets

def show(app):
    existing = getattr(app, 'overlay_settings_window', None)
    if existing is not None and existing.winfo_exists():
        existing.lift(); return
    model = app.overlay.settings
    win = tk.Toplevel(app.root); app.overlay_settings_window = win
    win.title('Settings — overlay placement'); win.transient(app.root)
    from responsive_ui import scroll_surface,fit_window
    fit_window(win,560,730)
    surface=scroll_surface(win,520,650)
    frame = ttk.Frame(surface, padding=18); frame.pack(fill='both', expand=True)
    ttk.Label(frame, text='OVERLAY POSITION', style='Eyebrow.TLabel').pack(anchor='w')
    ttk.Label(frame, text='Drag the cyan box, or use the sliders. Changes preview live.\nPositions follow the game window at any resolution.').pack(anchor='w', pady=8)
    values = [tk.DoubleVar(value=v) for v in model.current]
    original = list(model.current)
    presets = dict(model.presets)
    preview = tk.Canvas(frame, width=480, height=230, bg='#09131d', highlightthickness=0)
    preview.pack(fill='x', pady=8)
    caption = tk.StringVar()
    def redraw(*args):
        model.current = [v.get() for v in values]
        preview.delete('all')
        rect = app.overlay.game_rect() or (0, 0, 1920, 1080)
        _, _, gw, gh = rect
        scale = min(460/gw, 210/gh)
        pw, ph = gw*scale, gh*scale
        left, top = (480-pw)/2, (230-ph)/2
        preview.create_rectangle(left,top,left+pw,top+ph,outline='#526b7b')
        x,y,w,h = placement((0,0,gw,gh),model.current)
        preview.create_rectangle(left+x*scale,top+y*scale,left+(x+w)*scale,top+(y+h)*scale,outline='#68dfeb',fill='#244e62')
        preview.create_text(left+(x+w/2)*scale,top+(y+h/2)*scale,text='OVERLAY',fill='#e2edf2',font=('Segoe UI',8))
        caption.set(f'Horizontal {values[0].get():.0f}%   •   Vertical {values[1].get():.0f}%   •   Preview {gw} × {gh}')
    def drag(event):
        _,_,gw,gh = app.overlay.game_rect() or (0,0,1920,1080)
        scale = min(460/gw,210/gh); left=(480-gw*scale)/2; top=(230-gh*scale)/2
        for variable, cursor, start, size, box in zip(values,(event.x,event.y),(left,top),(gw,gh),(460,265)):
            travel=max(1,size-min(box,size))
            variable.set(max(0,min(100,((cursor-start)/scale-min(box,size)/2)/travel*100)))
    preview.bind('<Button-1>',drag); preview.bind('<B1-Motion>',drag)
    ttk.Label(frame,textvariable=caption,style='Muted.TLabel').pack(anchor='w')
    for label,var in zip(('Horizontal: left → right','Vertical: top → bottom'), values):
        ttk.Label(frame,text=label).pack(anchor='w',pady=(8,0))
        ttk.Scale(frame,from_=0,to=100,variable=var).pack(fill='x')
        var.trace_add('write',redraw)
    def set_position(p):
        for var,value in zip(values,p):var.set(value)
    ttk.Button(frame,text='Center overlay',command=lambda:set_position([50,50])).pack(anchor='w',pady=10)
    ttk.Label(frame,text='SHIP / POSITION PRESETS',style='Eyebrow.TLabel').pack(anchor='w',pady=(6,8))
    selected=tk.StringVar()
    combo=ttk.Combobox(frame,textvariable=selected,state='readonly',values=sorted(presets));combo.pack(fill='x')
    combo.bind('<<ComboboxSelected>>',lambda e:set_position(presets[selected.get()]))
    row=ttk.Frame(frame);row.pack(fill='x',pady=8)
    def save_preset():
        name=simpledialog.askstring('Save preset','Preset name (for example, Prospector):',initialvalue=selected.get(),parent=win)
        if not name or not name.strip():return
        name=name.strip()
        if name in presets and not messagebox.askyesno('Replace preset',f'Replace “{name}”?',parent=win):return
        presets[name]=[v.get() for v in values];combo.configure(values=sorted(presets));selected.set(name)
    def delete():
        name=selected.get()
        if name in presets and messagebox.askyesno('Delete preset',f'Delete “{name}”?',parent=win):
            del presets[name];combo.configure(values=sorted(presets));selected.set('')
    ttk.Button(row,text='Save as preset…',command=save_preset).pack(side='left')
    ttk.Button(row,text='Delete preset',command=delete).pack(side='left',padx=8)
    ttk.Label(frame,text='Save settings keeps this position and your presets for next launch.\nLive preview appears when a bookmark and flight overlay are active.',style='Muted.TLabel').pack(anchor='w',pady=8)
    def cancel():model.current=original;win.destroy()
    def save():
        try:model.save([v.get() for v in values],presets)
        except OSError as e:messagebox.showerror('Settings',f'Could not save settings: {e}',parent=win);return
        app.status.set('Overlay position and presets saved.');win.destroy()
    row=ttk.Frame(frame);row.pack(fill='x',pady=(8,0))
    ttk.Button(row,text='Save settings',style='Primary.TButton',command=save).pack(side='right')
    ttk.Button(row,text='Cancel',command=cancel).pack(side='right',padx=8)
    win.protocol('WM_DELETE_WINDOW',cancel);redraw()
