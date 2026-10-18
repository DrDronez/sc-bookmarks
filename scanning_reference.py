"""Offline scan-signature reference using bundled CSV tables."""
import csv,re
from pathlib import Path
import tkinter as tk
from tkinter import ttk
DATA=Path(__file__).resolve().parent/'scanning-data'
CHARTS={'Mining — Space':'mining-space.csv','Mining — Planetary Surface':'mining-planetary-surface.csv','Salvage':'salvage.csv'}

def load_charts(directory=DATA):
    charts={}
    for name,filename in CHARTS.items():
        with (directory/filename).open(encoding='utf-8-sig',newline='') as stream:
            records=list(csv.reader(stream))
        header,*rows=records
        if len(header)!=(2 if name=='Salvage' else 7):raise ValueError('Unexpected columns in '+filename)
        parsed=[]
        for row in rows:
            if len(row)!=len(header):raise ValueError('Incomplete row in '+filename)
            parsed.append((int(row[0]),row[1]) if name=='Salvage' else (row[0],*map(int,row[1:])))
        charts[name]={'headers':header,'rows':parsed}
    return charts

def parse_rs(value):
    value=value.strip().replace(',','').replace(' ','')
    if not value:return None
    if not re.fullmatch(r'\d+',value) or int(value)<=0:raise ValueError('Enter a positive whole RS reading, such as 18,000.')
    return int(value)

def matching_columns(chart,row,rs):
    if rs is None:return []
    if chart=='Salvage':return [0] if row[0]==rs else []
    return [i for i in range(1,len(row)) if row[i]==rs]

def filter_rows(chart,rows,query='',rs=None):
    query=query.strip().casefold()
    return [row for row in rows if (not query or query in str(row[1] if chart=='Salvage' else row[0]).casefold()) and (rs is None or matching_columns(chart,row,rs))]

class ScanningTab(ttk.Frame):
    def __init__(self,parent):
        super().__init__(parent,padding=14)
        self.charts=load_charts();self.mode=tk.StringVar(value='Mining — Space');self.rs=tk.StringVar();self.search=tk.StringVar()
        self.summary=tk.StringVar();self.note=tk.StringVar();self.selection_info=tk.StringVar(value='')
        self.sort_column=None;self.descending=False
        ttk.Label(self,text='SCAN SIGNATURE REFERENCE',style='Eyebrow.TLabel').pack(anchor='w',pady=(0,10))
        row=ttk.Frame(self);row.pack(fill='x')
        ttk.Combobox(row,textvariable=self.mode,values=list(CHARTS),state='readonly',width=29).pack(side='left')
        ttk.Label(row,text='RS reading',style='Muted.TLabel').pack(side='left',padx=(16,6))
        ttk.Entry(row,textvariable=self.rs,width=12).pack(side='left')
        ttk.Label(row,text='Ore / contact',style='Muted.TLabel').pack(side='left',padx=(16,6))
        ttk.Entry(row,textvariable=self.search,width=18).pack(side='left',fill='x',expand=True)
        ttk.Button(row,text='Clear',command=self.clear).pack(side='left',padx=(8,0))
        ttk.Label(self,textvariable=self.summary,foreground='#68dfeb',wraplength=970).pack(anchor='w',pady=(10,6))
        holder=ttk.Frame(self);holder.pack(fill='both',expand=True)
        self.table=ttk.Treeview(holder,show='headings',height=9,selectmode='browse',style='Scanning.Treeview')
        self.table.grid(row=0,column=0,sticky='nsew');holder.columnconfigure(0,weight=1);holder.rowconfigure(0,weight=1)
        vertical=ttk.Scrollbar(holder,orient='vertical',command=self.table.yview);vertical.grid(row=0,column=1,sticky='ns')
        horizontal=ttk.Scrollbar(holder,orient='horizontal',command=self.table.xview);horizontal.grid(row=1,column=0,sticky='ew')
        self.table.configure(yscrollcommand=vertical.set,xscrollcommand=horizontal.set)
        self.table.tag_configure('alternate',background='#142734');self.table.tag_configure('match',foreground='#efbd70')
        ttk.Label(self,textvariable=self.selection_info,style='Muted.TLabel',wraplength=970).pack(anchor='w',pady=(6,0))
        ttk.Label(self,textvariable=self.note,style='Muted.TLabel',wraplength=970,justify='left').pack(anchor='w',pady=(6,0))
        foot=ttk.Frame(self);foot.pack(fill='x',pady=(8,0))
        ttk.Label(foot,text='Alpha 4.10.2 • Offline scanning reference',style='Muted.TLabel').pack(side='left')
        self.table.bind('<<TreeviewSelect>>',lambda e:self.describe())
        self.mode.trace_add('write',lambda *a:self.change_chart())
        self.rs.trace_add('write',lambda *a:self.refresh());self.search.trace_add('write',lambda *a:self.refresh())
        self.change_chart()
    def clear(self):self.rs.set('');self.search.set('')
    def change_chart(self):
        self.sort_column=None;self.descending=False
        chart=self.mode.get();headers=self.charts[chart]['headers'];cols=[str(i) for i in range(len(headers))]
        self.table.configure(columns=cols)
        for i,label in enumerate(headers):
            self.table.heading(str(i),text=label,command=lambda index=i:self.sort(index))
            width=(125 if i==0 else 600) if chart=='Salvage' else (175 if i==0 else 105)
            self.table.column(str(i),width=width,minwidth=width,anchor='w' if chart=='Salvage' or i==0 else 'e',stretch=True)
        self.note.set('Listed salvage possibilities; scrap count does not indicate panel size or yield. Wreck values refer to spawned salvage contacts. Other contact types can share an RS reading.' if chart=='Salvage' else 'RS = base signature × contributing rocks of the same primary ore (1–6). These totals include overlapping clusters, not guaranteed spawn sizes. Mixed contacts may produce other totals; RS does not reveal ore quality or yield.'+(' Hand/ROC rows identify deposit count, not mineral type.' if 'Surface' in chart else ''))
        self.refresh()
    def sort(self,column):
        self.descending=not self.descending if self.sort_column==column else False
        self.sort_column=column;self.refresh()
    def refresh(self):
        self.table.delete(*self.table.get_children());self.selection_info.set('')
        chart=self.mode.get()
        try:rs=parse_rs(self.rs.get())
        except ValueError as e:self.summary.set(str(e));return
        self.visible=filter_rows(chart,self.charts[chart]['rows'],self.search.get(),rs)
        if self.sort_column is not None:
            self.visible.sort(key=lambda row:row[self.sort_column].casefold() if isinstance(row[self.sort_column],str) else row[self.sort_column],reverse=self.descending)
        for i,row in enumerate(self.visible):
            self.table.insert('', 'end',iid=str(i),values=[f'{v:,}' if isinstance(v,int) else v for v in row],tags=(('alternate',) if i%2 else ())+ (('match',) if rs is not None else ()))
        if rs is None:self.summary.set(f'{len(self.visible)} entries • Enter an RS reading to find exact matches, or browse the full chart.')
        else:
            other=[name+': '+str(len(filter_rows(name,data['rows'],rs=rs))) for name,data in self.charts.items() if name!=chart and filter_rows(name,data['rows'],rs=rs)]
            self.summary.set(f'{rs:,} RS • {len(self.visible)} matching row(s) in this view.'+(' Also matches '+ '; '.join(other)+'.' if other else '')+(' No exact match here; mixed contacts may give other totals.' if not self.visible else ''))
            if self.visible:self.table.selection_set('0');self.describe()
    def describe(self):
        selected=self.table.selection()
        if not selected:return
        row=self.visible[int(selected[0])];chart=self.mode.get()
        try:rs=parse_rs(self.rs.get())
        except ValueError:return
        if chart=='Salvage':text=f'{row[0]:,} RS → {row[1]}'
        elif rs is None:text=f'{row[0]} • Base RS: {row[1]:,}. Read across for the number of contributing rocks.'
        else:text=f'{rs:,} RS → '+', '.join(f'{n} '+('rock' if n==1 else 'rocks')+' / deposits' for n in matching_columns(chart,row,rs))+f' • {row[0]}'
        self.selection_info.set(text)
