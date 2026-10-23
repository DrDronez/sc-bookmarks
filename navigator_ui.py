"""Original desktop styling and layout for SC Bookmarks; no external visual assets."""
import tkinter as tk
from tkinter import ttk
from mining_bookmarks import COMPOSITION_TYPES

BG='#0b121a';PANEL='#111f2b';FIELD='#09131d';EDGE='#29404e';TEXT='#e2edf2';MUTED='#92aab8';CYAN='#68dfeb';AMBER='#efbd70'

def bookmark_summary(bookmark):
    from datetime import datetime,timezone
    saved='unknown'
    try:
        stamp=datetime.fromisoformat(bookmark['utc'].replace('Z','+00:00'))
        if stamp.tzinfo is None:stamp=stamp.replace(tzinfo=timezone.utc)
        saved=stamp.astimezone().strftime('%Y-%m-%d %I:%M:%S %p')+' (local)'
    except (KeyError,ValueError,TypeError,AttributeError,OverflowError):pass
    return ('System: '+bookmark.get('system','unknown')+' • Created on shard: '+(bookmark.get('shard_id') or 'unknown')+'\nSaved: '+saved)

def theme(root):
    root.configure(background=BG)
    root.option_add('*Font',('Segoe UI',10))
    for key,value in [('Background',PANEL),('Foreground',TEXT),('insertBackground',CYAN),('selectBackground','#244e62'),('selectForeground','#ffffff')]:
        root.option_add('*'+key,value)
    root.option_add('*TCombobox*Listbox.background',FIELD)
    root.option_add('*TCombobox*Listbox.foreground',TEXT)
    s=ttk.Style(root);s.theme_use('clam')
    s.configure('.',background=PANEL,foreground=TEXT,font=('Segoe UI',10),bordercolor=EDGE,lightcolor=EDGE,darkcolor=EDGE)
    s.configure('TFrame',background=PANEL)
    s.configure('Shell.TFrame',background=BG)
    s.configure('TLabel',background=PANEL,foreground=TEXT)
    s.configure('Muted.TLabel',foreground=MUTED)
    s.configure('Title.TLabel',font=('Segoe UI Semibold',22),foreground=TEXT,background=BG)
    s.configure('Eyebrow.TLabel',foreground=CYAN,font=('Segoe UI Semibold',9))
    s.configure('TButton',padding=(12,7),background='#1b3040',foreground=TEXT,borderwidth=1,focusthickness=1,focuscolor=CYAN)
    s.map('TButton',background=[('active','#294b60'),('pressed','#345d70')],foreground=[('disabled','#637b88')])
    s.configure('Primary.TButton',background=CYAN,foreground='#07131d',font=('Segoe UI Semibold',10))
    s.map('Primary.TButton',background=[('active','#9cecf3'),('pressed','#49bfcc')],foreground=[('disabled','#516c77')])
    s.configure('Amber.TButton',foreground=AMBER)
    s.configure('Danger.TButton',foreground='#f49a99')
    s.configure('TEntry',fieldbackground=FIELD,foreground=TEXT,insertcolor=CYAN,padding=6)
    s.configure('TCombobox',fieldbackground=FIELD,background='#1b3040',foreground=TEXT,arrowsize=13,padding=5)
    s.map('TCombobox',fieldbackground=[('readonly',FIELD)],foreground=[('readonly',TEXT)],selectbackground=[('readonly',FIELD)],selectforeground=[('readonly',TEXT)])
    s.configure('TCheckbutton',background=PANEL,foreground=MUTED,padding=(0,4))
    s.map('TCheckbutton',background=[('active',PANEL)],foreground=[('active',TEXT)])
    s.configure('TNotebook',background=BG,borderwidth=0)
    s.configure('TNotebook.Tab',padding=(18,9),background='#13232f',foreground=MUTED)
    s.map('TNotebook.Tab',background=[('selected',PANEL)],foreground=[('selected',CYAN)])
    s.configure('TSeparator',background=EDGE)
    s.configure('Scanning.Treeview',background=FIELD,fieldbackground=FIELD,foreground=TEXT,rowheight=29,font=('Segoe UI',10),bordercolor=EDGE)
    s.configure('Scanning.Treeview.Heading',background='#1b3040',foreground=CYAN,padding=(8,8))
    s.map('Scanning.Treeview',background=[('selected','#24556a')],foreground=[('selected','#ffffff')])
    s.configure('Vertical.TScrollbar',background='#294553',troughcolor=FIELD,arrowcolor=MUTED)
    s.configure('Horizontal.TScrollbar',background='#294553',troughcolor=FIELD,arrowcolor=MUTED)

def build(app):
    root=app.root;theme(root)
    from responsive_ui import scroll_surface,fit_window
    fit_window(root,1380,940)
    viewport=scroll_surface(root)
    shell=ttk.Frame(viewport,style='Shell.TFrame',padding=20);shell.pack(fill='both',expand=True)
    head=ttk.Frame(shell,style='Shell.TFrame');head.pack(fill='x',pady=(0,16))
    ttk.Label(head,text='SC BOOKMARKS',style='Title.TLabel',font=('Segoe UI Semibold',22)).pack(side='left')
    from overlay_settings import show as show_settings
    ttk.Button(head,text='Settings',command=lambda:show_settings(app)).pack(side='right')
    capture=ttk.Frame(shell,padding=14);capture.pack(fill='x',pady=(0,16))
    ttk.Label(capture,text='01  /  CAPTURE A LOCATION',style='Eyebrow.TLabel').pack(anchor='w',pady=(0,8))
    row=ttk.Frame(capture);row.pack(fill='x')
    ttk.Label(row,text='Name',style='Muted.TLabel').pack(side='left',padx=(0,8))
    ttk.Entry(row,textvariable=app.name).pack(side='left',fill='x',expand=True,padx=(0,12))
    ttk.Label(row,text='Save to',style='Muted.TLabel').pack(side='left',padx=(0,6))
    app.destination_box=ttk.Combobox(row,textvariable=app.new_category,state='readonly',width=14)
    app.destination_box.pack(side='left')
    row=ttk.Frame(capture);row.pack(fill='x',pady=(10,0))
    ttk.Button(row,text='Bookmark here',style='Primary.TButton',command=app.bookmark).pack(side='left')
    ttk.Button(row,text='Bookmark asteroid',style='Amber.TButton',command=app.bookmark_asteroid).pack(side='left',padx=8)
    ttk.Button(row,text='Enter coordinates',command=app.manual_bookmark).pack(side='left')
    ttk.Button(row,text='Capture in 8s',command=app.schedule_bookmark).pack(side='left',padx=8)
    ttk.Button(row,text='Cancel timer',command=app.cancel_bookmark).pack(side='left')
    app.countdown_text.set('Asteroid scan: center the rock and keep SCAN RESULTS visible.')
    ttk.Label(capture,textvariable=app.countdown_text,style='Muted.TLabel').pack(anchor='w',pady=(7,0))

    main=ttk.Frame(shell,style='Shell.TFrame');main.pack(fill='both',expand=True)
    main.columnconfigure(0,weight=1);main.columnconfigure(1,weight=0,minsize=290);main.rowconfigure(0,weight=1)
    tabs=ttk.Notebook(main);tabs.grid(row=0,column=0,sticky='nsew',padx=(0,14))
    library=ttk.Frame(tabs,padding=14);settings=ttk.Frame(tabs,padding=16)
    from system_map import SystemMap
    app.map_tab=SystemMap(tabs,app);tabs.add(app.map_tab,text='Map')
    tabs.add(library,text='Bookmark library')
    from scanning_reference import ScanningTab
    app.scanning_tab=ScanningTab(tabs);tabs.add(app.scanning_tab,text='Scanning')
    tabs.add(settings,text='System & capture')
    scan_controls=ttk.Frame(app.scanning_tab);scan_controls.pack(fill='x',before=app.scanning_tab.winfo_children()[0],pady=(0,8))
    ttk.Checkbutton(scan_controls,text='Identify RS contacts in game (fast updates)',variable=app.scan_labels_enabled).pack(anchor='w')
    ttk.Button(scan_controls,text='Start capture',command=app.start).pack(anchor='w',pady=4)
    ttk.Label(scan_controls,textvariable=app.scan_label_status,style='Muted.TLabel').pack(anchor='w')
    row=ttk.Frame(library);row.pack(fill='x')
    ttk.Label(row,text='Category',style='Muted.TLabel').pack(side='left')
    app.category_box=ttk.Combobox(row,textvariable=app.category_filter,state='readonly',width=16)
    app.category_box.pack(side='left',padx=(6,12));app.category_box.bind('<<ComboboxSelected>>',lambda e:app.refresh_bookmarks())
    ttk.Label(row,text='Sort',style='Muted.TLabel').pack(side='left')
    ttk.Combobox(row,textvariable=app.sort_order,state='readonly',values=['Category','Name','System','Newest','Ore type','Ore quality'],width=12).pack(side='left',padx=6)
    ttk.Checkbutton(library,text='Only show the current system',variable=app.hide_other_systems,command=app.organization_changed).pack(anchor='w',pady=4)
    row=ttk.Frame(library);row.pack(fill='x',pady=(0,10))
    ttk.Combobox(row,textvariable=app.ore_filter,values=['Any ore']+COMPOSITION_TYPES,state='readonly',width=16).pack(side='left')
    ttk.Label(row,text='Quality ≥',style='Muted.TLabel').pack(side='left',padx=6)
    ttk.Entry(row,textvariable=app.quality_filter,width=6).pack(side='left')
    ttk.Button(row,text='Find rocks',command=app.apply_mining_filter).pack(side='left',padx=6)
    ttk.Button(row,text='Clear',command=app.clear_mining_filter).pack(side='left')
    list_frame=ttk.Frame(library);list_frame.pack(fill='both',expand=True)
    app.list=tk.Listbox(list_frame,height=7,exportselection=False,background=FIELD,foreground=TEXT,selectbackground='#24556a',selectforeground='#ffffff',font=('Segoe UI',11),borderwidth=0,highlightthickness=1,highlightbackground=EDGE,highlightcolor=CYAN,activestyle='none')
    app.list.pack(side='left',fill='both',expand=True)
    scroll=ttk.Scrollbar(list_frame,command=app.list.yview);scroll.pack(side='right',fill='y');app.list.configure(yscrollcommand=scroll.set)
    horizontal=ttk.Scrollbar(library,orient='horizontal',command=app.list.xview);horizontal.pack(fill='x');app.list.configure(xscrollcommand=horizontal.set)
    row=ttk.Frame(library);row.pack(fill='x',pady=(10,5))
    ttk.Button(row,text='Edit details / notes',command=app.edit_bookmark).pack(side='left')
    ttk.Button(row,text='Rename',command=app.rename_bookmark).pack(side='left',padx=6)
    ttk.Button(row,text='Move to category',command=app.move_category).pack(side='left')
    ttk.Button(row,text='Delete',style='Danger.TButton',command=app.delete_bookmark).pack(side='right')
    row=ttk.Frame(library);row.pack(fill='x')
    ttk.Button(row,text='Copy link',command=app.copy_bookmarks).pack(side='left')
    ttk.Button(row,text='Copy all',command=lambda:app.copy_bookmarks(all_bookmarks=True)).pack(side='left',padx=6)
    ttk.Button(row,text='Paste / import',command=app.import_bookmarks).pack(side='left')
    ttk.Label(library,textvariable=app.bookmark_details,style='Muted.TLabel',wraplength=570).pack(anchor='w',pady=(8,0))
    app.list.bind('<<ListboxSelect>>',lambda event:app.select());app.list.bind('<F2>',lambda event:app.rename_bookmark());app.list.bind('<Delete>',lambda event:app.delete_bookmark())
    def edit_clicked(event):
        index=app.list.nearest(event.y)
        bounds=app.list.bbox(index)
        if bounds and bounds[1]<=event.y<bounds[1]+bounds[3]:
            app.list.selection_clear(0,'end');app.list.selection_set(index);app.select();app.edit_bookmark()
        return 'break'
    app.list.bind('<Double-Button-1>',edit_clicked)

    nav=ttk.Frame(main,padding=16,width=290);nav.grid(row=0,column=1,sticky='nsew');nav.grid_propagate(False)
    ttk.Label(nav,text='02  /  FLIGHT GUIDANCE',style='Eyebrow.TLabel').pack(anchor='w')
    ttk.Label(nav,textvariable=app.nav_text,font=('Consolas',11),wraplength=258,justify='left').pack(anchor='w',fill='x',pady=(14,12))
    def navigate():
        app.select()
        if app.target is not None and not app.running:app.start()
    ttk.Button(nav,text='Navigate to selected',style='Primary.TButton',command=navigate).pack(fill='x')
    ttk.Button(nav,text='Clear target / hide overlay',command=app.clear_target).pack(fill='x',pady=(7,12))
    ttk.Checkbutton(nav,text='Show flight overlay',variable=app.overlay_enabled).pack(anchor='w')
    ttk.Separator(nav).pack(fill='x',pady=14)
    ttk.Label(nav,text='QUANTUM APPROACH',style='Eyebrow.TLabel').pack(anchor='w',pady=(0,8))
    row=ttk.Frame(nav);row.pack(fill='x')
    ttk.Label(row,text='Alert lead (seconds)',style='Muted.TLabel').pack(side='left')
    ttk.Entry(row,textvariable=app.lead,width=5).pack(side='right')
    ttk.Button(nav,text='Arm / disarm exit alert',command=app.arm).pack(fill='x',pady=10)
    ttk.Label(nav,text='An audible cue tells you when to exit QT. You control the ship.',style='Muted.TLabel',wraplength=252).pack(anchor='w')

    ttk.Label(settings,text='SYSTEM & SESSION',style='Eyebrow.TLabel').pack(anchor='w',pady=(0,10))
    row=ttk.Frame(settings);row.pack(fill='x')
    ttk.Label(row,text='Current system').pack(side='left',padx=(0,10));ttk.Entry(row,textvariable=app.system_name,width=18).pack(side='left')
    ttk.Checkbutton(settings,text='Detect system automatically (every 45 seconds)',variable=app.auto_system,command=app.save_organization).pack(anchor='w',pady=7)
    ttk.Label(settings,textvariable=app.shard_text,style='Muted.TLabel',wraplength=560).pack(anchor='w')
    ttk.Button(settings,text='Manage categories',command=app.manage_categories).pack(anchor='w',pady=10)
    def enable_links():
        from scbm_protocol import register
        from tkinter import messagebox
        try:register();app.status.set('scbm:// links are enabled for this Windows account.')
        except (OSError,ValueError) as e:messagebox.showerror('Enable bookmark links',str(e),parent=root)
    ttk.Button(settings,text='Enable scbm:// links in Windows',command=enable_links).pack(anchor='w',pady=(0,8))

    ttk.Separator(settings).pack(fill='x',pady=8)
    ttk.Label(settings,text='CAPTURE STATUS',style='Eyebrow.TLabel').pack(anchor='w',pady=8)
    ttk.Label(settings,textvariable=app.pos_text,font=('Consolas',10),wraplength=560).pack(anchor='w')
    ttk.Label(settings,textvariable=app.reading_age,style='Muted.TLabel',wraplength=560).pack(anchor='w',pady=8)
    row=ttk.Frame(settings);row.pack(fill='x',pady=4)
    ttk.Button(row,text='Start capture',command=app.start).pack(side='left')
    ttk.Button(row,text='Stop capture',command=app.halt).pack(side='left',padx=6)
    ttk.Button(settings,text='Save last reading (may be stale)',command=lambda:app.bookmark(cached=True)).pack(anchor='w',pady=8)
    ttk.Label(settings,text='Keep the cockpit camera centered for guidance.\nFreelook changes the camera direction.',style='Muted.TLabel').pack(anchor='w',pady=6)
    def tab_changed(event):
        scanning=tabs.select()==str(app.scanning_tab)
        mapping=tabs.select()==str(app.map_tab)
        if mapping:capture.pack_forget()
        else:capture.pack(fill='x',pady=(0,16),before=main)
        if scanning or mapping:
            nav.grid_remove();main.columnconfigure(1,minsize=0);tabs.grid_configure(columnspan=2,padx=0)
        else:
            nav.grid();main.columnconfigure(1,minsize=290);tabs.grid_configure(columnspan=1,padx=(0,14))
    tabs.bind('<<NotebookTabChanged>>',tab_changed)
    app.refresh_bookmarks()
    app.system_name.trace_add('write',lambda *a:app.organization_changed());app.sort_order.trace_add('write',lambda *a:app.organization_changed())
    footer=ttk.Frame(shell,style='Shell.TFrame');footer.pack(side='bottom',fill='x',pady=(12,0),before=main)
    ttk.Label(footer,textvariable=app.status,foreground=MUTED,background=BG,wraplength=970).pack(anchor='w')
