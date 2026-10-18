import copy
import tkinter as tk
from tkinter import ttk, simpledialog, messagebox
from bookmark_links import validate
from mining_bookmarks import COMPOSITION_TYPES, number, clean_mining, estimated_scu

class BookmarkEditor(simpledialog.Dialog):
    def __init__(self,parent,bookmark,categories,on_save):
        self.bookmark=copy.deepcopy(bookmark);self.categories=categories;self.on_save=on_save
        self.rows=[];self.fields={};self.mining_fields={}
        super().__init__(parent,title='Bookmark details')

    def body(self,master):
        self.tabs=ttk.Notebook(master);self.tabs.pack(fill='both',expand=True)
        general=ttk.Frame(self.tabs,padding=10);self.mining=ttk.Frame(self.tabs,padding=10)
        self.tabs.add(general,text='Bookmark & notes');self.tabs.add(self.mining,text='Mining')
        b=self.bookmark;xyz=b.get('xyz_km',['','',''])
        fields=[('name','Name',b.get('name','')),('category','Category',b.get('category','Personal')),
                ('system','System',b.get('system','')),('x','Root X (km)',xyz[0]),
                ('y','Root Y (km)',xyz[1]),('z','Root Z (km)',xyz[2]),
                ('shard_id','Created on shard (optional)',b.get('shard_id') or '')]
        for i,(key,label,value) in enumerate(fields):
            ttk.Label(general,text=label).grid(row=i,column=0,sticky='w',padx=4,pady=3)
            var=tk.StringVar(value=str(value));self.fields[key]=var
            if key=='category':entry=ttk.Combobox(general,textvariable=var,values=self.categories,state='readonly',width=42)
            else:entry=ttk.Entry(general,textvariable=var,width=45)
            entry.grid(row=i,column=1,sticky='ew',padx=4,pady=3)
        ttk.Label(general,text='Notes').grid(row=7,column=0,sticky='nw',padx=4,pady=5)
        self.notes=tk.Text(general,width=55,height=7,wrap='word');self.notes.grid(row=7,column=1,pady=5)
        self.notes.insert('1.0',b.get('notes',''))
        ttk.Label(general,text='Shard is informational; it never restricts navigation.').grid(row=8,column=0,columnspan=2,sticky='w')
        data=b.get('mining',{})
        stats=ttk.Frame(self.mining);stats.pack(fill='x')
        for i,(key,label) in enumerate([('mass','Mass'),('resistance','Resistance (%)'),('instability','Instability'),('scu','Rock SCU')]):
            ttk.Label(stats,text=label).grid(row=i//2,column=(i%2)*2,sticky='w',padx=5,pady=4)
            var=tk.StringVar(value='' if data.get(key) is None else str(data[key]));self.mining_fields[key]=var
            ttk.Entry(stats,textvariable=var,width=16).grid(row=i//2,column=(i%2)*2+1,padx=5,pady=4)
        ttk.Label(self.mining,text='Composition — repeated ore types are allowed').pack(anchor='w',pady=(12,4))
        header=ttk.Frame(self.mining);header.pack(fill='x')
        for label,width in [('Ore type',23),('Composition %',16),('Quality',13),('Estimated SCU',17)]:
            ttk.Label(header,text=label,width=width).pack(side='left')
        holder=ttk.Frame(self.mining);holder.pack(fill='both',expand=True)
        canvas=tk.Canvas(holder,width=650,height=230,highlightthickness=0)
        scroll=ttk.Scrollbar(holder,orient='vertical',command=canvas.yview)
        canvas.configure(yscrollcommand=scroll.set);canvas.pack(side='left',fill='both',expand=True);scroll.pack(side='right',fill='y')
        self.row_body=ttk.Frame(canvas);self.canvas=canvas
        canvas.create_window((0,0),window=self.row_body,anchor='nw')
        self.row_body.bind('<Configure>',lambda e:canvas.configure(scrollregion=canvas.bbox('all')))
        for row in data.get('composition',[]):self.add_row(row)
        ttk.Button(self.mining,text='+ Add ore entry',command=self.add_row).pack(anchor='w',pady=6)
        ttk.Label(self.mining,text='Estimated SCU = rock SCU × composition % ÷ 100.\nBlank until rock SCU, composition %, and quality are entered.').pack(anchor='w')
        self.mining_fields['scu'].trace_add('write',lambda *a:self.recalculate())
        self.fields['category'].trace_add('write',lambda *a:self.category_changed())
        self.category_changed();self.recalculate()
        if self.fields['category'].get()=='Mining':self.tabs.select(self.mining)
        return None

    def category_changed(self):
        enabled=self.fields['category'].get()=='Mining'
        if not enabled:self.tabs.select(0)
        self.tabs.tab(self.mining,state='normal' if enabled else 'disabled')

    def add_row(self,data=None):
        if len(self.rows)>=100:return messagebox.showinfo('Composition','Maximum 100 entries.',parent=self)
        data=data or {};frame=ttk.Frame(self.row_body);frame.pack(fill='x',pady=3)
        row={'frame':frame}
        for key in ('ore','percent','quality'):
            row[key]=tk.StringVar(value='' if data.get(key) is None else str(data[key]))
        ttk.Combobox(frame,textvariable=row['ore'],values=COMPOSITION_TYPES,state='readonly',width=21).pack(side='left',padx=(0,6))
        for key in ('percent','quality'):ttk.Entry(frame,textvariable=row[key],width=13).pack(side='left',padx=4)
        row['estimate']=tk.StringVar()
        ttk.Entry(frame,textvariable=row['estimate'],state='readonly',width=15).pack(side='left',padx=4)
        ttk.Button(frame,text='Remove',command=lambda:self.remove_row(row)).pack(side='left',padx=4)
        self.rows.append(row)
        for key in ('percent','quality'):row[key].trace_add('write',lambda *a:self.recalculate())
        self.recalculate()

    def remove_row(self,row):
        self.rows.remove(row);row['frame'].destroy()

    def recalculate(self):
        for row in self.rows:
            try:
                scu=number(self.mining_fields['scu'].get(),'SCU')
                entry={'percent':number(row['percent'].get(),'Composition %',100),'quality':number(row['quality'].get(),'Quality')}
                value=estimated_scu(scu,entry)
            except ValueError:value=None
            row['estimate'].set('' if value is None else f'{value:,.3f}')

    def candidate(self):
        b=copy.deepcopy(self.bookmark)
        b.update({key:self.fields[key].get().strip() for key in ('name','category','system','shard_id')})
        b['xyz_km']=[self.fields[key].get().strip() for key in ('x','y','z')]
        b['notes']=self.notes.get('1.0','end-1c')
        if b['category']=='Mining':
            data={key:var.get().strip() for key,var in self.mining_fields.items()}
            data['composition']=[]
            for row in self.rows:
                item={key:row[key].get().strip() for key in ('ore','percent','quality')}
                if any(item.values()):data['composition'].append(item)
            b['mining']=clean_mining(data)
        return validate(b)

    def validate(self):
        try:self.on_save(self.candidate());return True
        except (ValueError,OSError) as e:
            messagebox.showerror('Save bookmark',str(e),parent=self);return False
