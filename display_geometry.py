"""Window geometry in physical pixels, shared by capture and overlay."""
import ctypes
from ctypes import wintypes

def enable_dpi_awareness():
    try:ctypes.windll.user32.SetProcessDpiAwarenessContext(ctypes.c_void_p(-4))
    except (AttributeError,OSError):pass

def capture_client_box(width,height):
    user=ctypes.windll.user32
    user.FindWindowW.argtypes=[wintypes.LPCWSTR,wintypes.LPCWSTR]
    user.FindWindowW.restype=wintypes.HWND
    user.GetClientRect.argtypes=[wintypes.HWND,ctypes.POINTER(wintypes.RECT)]
    user.ClientToScreen.argtypes=[wintypes.HWND,ctypes.POINTER(wintypes.POINT)]
    user.GetWindowRect.argtypes=[wintypes.HWND,ctypes.POINTER(wintypes.RECT)]
    ctypes.windll.dwmapi.DwmGetWindowAttribute.argtypes=[wintypes.HWND,wintypes.DWORD,ctypes.c_void_p,wintypes.DWORD]
    hwnd=user.FindWindowW(None,'Star Citizen ') or user.FindWindowW(None,'Star Citizen')
    if not hwnd:return (0,0,width,height)
    client=wintypes.RECT();origin=wintypes.POINT()
    if not user.GetClientRect(hwnd,ctypes.byref(client)) or not user.ClientToScreen(hwnd,ctypes.byref(origin)):return (0,0,width,height)
    cw,ch=client.right,client.bottom
    if abs(width-cw)<=2 and abs(height-ch)<=2:return (0,0,width,height)
    outer=wintypes.RECT()
    try:result=ctypes.windll.dwmapi.DwmGetWindowAttribute(hwnd,9,ctypes.byref(outer),ctypes.sizeof(outer))
    except OSError:result=-1
    if result!=0:user.GetWindowRect(hwnd,ctypes.byref(outer))
    if abs(width-(outer.right-outer.left))>3 or abs(height-(outer.bottom-outer.top))>3:return (0,0,width,height)
    x,y=origin.x-outer.left,origin.y-outer.top
    if 0<=x<x+cw<=width and 0<=y<y+ch<=height:return x,y,x+cw,y+ch
    return (0,0,width,height)

def debug_box(width,height):
    # Reserve enough area for scaled debug fonts; OCR locates rows within this region.
    return max(0,width-max(700,round(width*.55))),0,width,min(height,max(310,round(height*.35)))
