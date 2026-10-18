"""Isolate the native capture runtime from Tk and its interpreter lock."""
import multiprocessing as mp
import queue,time
from types import SimpleNamespace

def capture_process(out,stop):
 from display_geometry import enable_dpi_awareness
 enable_dpi_awareness()
 from windows_capture import WindowsCapture
 capture=WindowsCapture(window_name='Star Citizen',cursor_capture=False,minimum_update_interval=500)
 last=[0.]
 @capture.event
 def on_frame_arrived(frame,control):
  now=time.monotonic()
  if now-last[0]<.9:return
  last[0]=now
  try:
   if not out.full():out.put_nowait((now,frame.frame_buffer[:,:,:3].copy()))
  except queue.Full:pass
 @capture.event
 def on_closed():stop.set()
 control=None
 try:
  control=capture.start_free_threaded()
  stop.wait()
 finally:
  if control is not None:control.stop()

class CaptureProcess:
 def __init__(self):
  ctx=mp.get_context('spawn');self.queue=ctx.Queue(maxsize=1);self.event=ctx.Event()
  self.process=ctx.Process(target=capture_process,args=(self.queue,self.event),daemon=True)
  self.process.start()
 def read(self):
  if not self.process.is_alive():raise RuntimeError('Game capture process exited. Restore the game window and start capture again.')
  stamp,pixels=self.queue.get(timeout=.5)
  return SimpleNamespace(width=pixels.shape[1],height=pixels.shape[0],frame_buffer=pixels,stamp=stamp)
 def stop(self):
  self.event.set();self.process.join(timeout=1)
  if self.process.is_alive():self.process.terminate();self.process.join(timeout=1)
  self.queue.close()
