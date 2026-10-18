"""Scrollable desktop surfaces for small screens and large Windows text scaling."""
import tkinter as tk
from tkinter import ttk

def scroll_surface(parent,min_width=1000,min_height=780):
    host=ttk.Frame(parent);host.pack(fill='both',expand=True)
    host.rowconfigure(0,weight=1);host.columnconfigure(0,weight=1)
    canvas=tk.Canvas(host,highlightthickness=0,bg='#0b121a')
    canvas.grid(row=0,column=0,sticky='nsew')
    vertical=ttk.Scrollbar(host,orient='vertical',command=canvas.yview);vertical.grid(row=0,column=1,sticky='ns')
    horizontal=ttk.Scrollbar(host,orient='horizontal',command=canvas.xview);horizontal.grid(row=1,column=0,sticky='ew')
    canvas.configure(xscrollcommand=horizontal.set,yscrollcommand=vertical.set)
    surface=ttk.Frame(canvas);item=canvas.create_window(0,0,window=surface,anchor='nw')
    def resize(event=None):
        width=max(min_width,surface.winfo_reqwidth(),canvas.winfo_width())
        height=max(min_height,surface.winfo_reqheight(),canvas.winfo_height())
        canvas.itemconfigure(item,width=width,height=height)
        canvas.configure(scrollregion=(0,0,width,height))
    canvas.bind('<Configure>',resize);surface.bind('<Configure>',resize)
    def wheel(event):
        widget=event.widget
        # Nested lists/text/tables keep their own wheel behavior.
        if widget.winfo_class() in ('Text','Listbox','Treeview','TCombobox'):return
        if widget.winfo_toplevel()!=parent.winfo_toplevel():return
        canvas.yview_scroll(-int(event.delta/120),'units')
    parent.bind('<MouseWheel>',wheel,add='+')
    return surface

def fit_window(window,width,height):
    scale=max(1,float(window.tk.call('tk','scaling'))/(96/72))
    width=round(width*scale);height=round(height*scale)
    window.geometry(f'{min(width,max(320,window.winfo_screenwidth()-60))}x{min(height,max(300,window.winfo_screenheight()-100))}')
    window.minsize(400,300)
