"""Windows per-user URI registration and atomic delivery to the desktop app."""
import ctypes,hashlib,json,os,subprocess,sys,uuid,shutil
from pathlib import Path
from ctypes import wintypes
from bookmark_links import decode,encode
BASE=Path(__file__).resolve().parent
INBOX=BASE/'navigator-data'/'incoming-links'

def acquire_instance():
    kernel=ctypes.WinDLL('kernel32',use_last_error=True)
    kernel.CreateMutexW.argtypes=[ctypes.c_void_p,wintypes.BOOL,wintypes.LPCWSTR]
    kernel.CreateMutexW.restype=wintypes.HANDLE
    name='Local\\SCBookmarks-'+hashlib.sha256((str(BASE)+str(Path.home())).encode()).hexdigest()[:24]
    handle=kernel.CreateMutexW(None,False,name)
    if not handle:raise ctypes.WinError(ctypes.get_last_error())
    if ctypes.get_last_error()==183:
        kernel.CloseHandle.argtypes=[wintypes.HANDLE];kernel.CloseHandle(handle)
        return None
    return handle

def register():
    import winreg
    pythonw=Path(sys.executable).with_name('pythonw.exe')
    if not pythonw.exists():raise OSError('Python windowed runtime not found.')
    command=f'"{pythonw}" "{BASE / "scbm_protocol.py"}" "%1"'
    keypath=r'Software\Classes\scbm'
    try:
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER,keypath+r'\shell\open\command') as key:
            old=winreg.QueryValueEx(key,None)[0]
            if old!=command and 'scbm_protocol.py' not in old:
                raise ValueError('scbm is already registered to another application; its registration was left unchanged.')
    except FileNotFoundError:pass
    with winreg.CreateKey(winreg.HKEY_CURRENT_USER,keypath) as key:
        winreg.SetValueEx(key,'',0,winreg.REG_SZ,'URL:SC Bookmarks')
        winreg.SetValueEx(key,'URL Protocol',0,winreg.REG_SZ,'')
    with winreg.CreateKey(winreg.HKEY_CURRENT_USER,keypath+r'\shell\open\command') as key:
        winreg.SetValueEx(key,'',0,winreg.REG_SZ,command)
    ctypes.windll.shell32.SHChangeNotify(0x08000000,0,None,None)
    return command

def deliver(link):
    # Canonicalize and validate before any files are written or processes started.
    canonical=encode(decode(link))
    INBOX.mkdir(parents=True,exist_ok=True)
    temp=INBOX/(uuid.uuid4().hex+'.tmp');final=temp.with_suffix('.json')
    temp.write_text(json.dumps({'uri':canonical}),encoding='utf-8');temp.replace(final)
    pythonw=Path(sys.executable).with_name('pythonw.exe')
    command=[str(pythonw),str(BASE/'SCBookmarks.py')]
    command.append('--no-capture')
    subprocess.Popen(command,cwd=str(BASE),stdin=subprocess.DEVNULL,stdout=subprocess.DEVNULL,stderr=subprocess.DEVNULL)

def duplicate(bookmarks,incoming):
    key=encode(incoming)
    for bookmark in bookmarks:
        try:
            if encode(bookmark)==key:return bookmark
        except (ValueError,TypeError):continue
    return None

def poll(app):
    try:
        if app.root.grab_current() is None:
            for path in sorted(INBOX.glob('*.json'))[:5]:
                try:
                    if path.stat().st_size>800000:raise ValueError('Incoming bookmark is too large.')
                    incoming=decode(json.loads(path.read_text(encoding='utf-8'))['uri'])
                    bookmark=duplicate(app.bookmarks,incoming)
                    if bookmark is None:app.add_bookmarks([incoming]);bookmark=incoming
                    app.category_filter.set('All categories');app.clear_mining_filter()
                    if app.hide_other_systems.get() and bookmark['system'].casefold()!=app.system_name.get().casefold():
                        app.hide_other_systems.set(False)
                    app.focus_bookmark(bookmark)
                    app.root.deiconify();app.root.lift()
                    app.status.set('Opened shared bookmark: '+bookmark['name']+(' — Navigate to selected starts live capture.' if not app.running else ''))
                    path.unlink()
                except (ValueError,TypeError,KeyError,OSError) as e:
                    app.status.set('Could not open shared bookmark: '+str(e))
                    # Keep failed deliveries available for diagnosis without retrying forever.
                    try:path.replace(path.with_suffix('.failed'))
                    except OSError:pass
    finally:
        if app.root.winfo_exists():app.root.after(500,lambda:poll(app))

if __name__=='__main__':
    try:
        if len(sys.argv)!=2:raise ValueError('Expected one scbm:// bookmark link.')
        if sys.argv[1]=='--register':register()
        else:deliver(sys.argv[1])
    except Exception as e:
        ctypes.windll.user32.MessageBoxW(None,str(e),'SC Bookmarks link',0x10)
        sys.exit(1)
