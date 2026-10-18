import importlib.util, unittest
from unittest.mock import Mock
import SCBookmarks as n
class TravelTests(unittest.TestCase):
 def test_mode_transition_and_stop(self):
  m=n.TravelMotion()
  for t in range(4):m.add(t,(t*.2,0,0))
  self.assertFalse(m.quantum)
  m.add(4,(100,0,0));self.assertFalse(m.quantum)
  m.add(5,(200,0,0));self.assertTrue(m.quantum)
  m.add(6,(200,0,0));self.assertTrue(m.quantum)
  m.add(7,(200,0,0));self.assertFalse(m.quantum);self.assertEqual(m.velocity,(0,0,0))
 def test_gap_retains_mode_but_disables_fresh_prediction(self):
  m=n.TravelMotion()
  for t in range(4):m.add(t,(100*t,0,0))
  self.assertTrue(m.quantum);self.assertFalse(m.fresh(10))
  m.add(10,(300,0,0));self.assertFalse(m.fresh(10));self.assertTrue(m.quantum)
 def test_eta(self):
  self.assertIn('01:40',n.arrival_text(10,.1,.1,True))
  self.assertIn('moving away',n.arrival_text(10,-.1,.1,True))
  self.assertIn('stopped',n.arrival_text(10,0,0,True))
  self.assertIn('paused',n.arrival_text(10,.1,.1,False))
 def test_qt_render_never_draws_arrows(self):
  overlay=n.FlightOverlay.__new__(n.FlightOverlay)
  overlay.user=Mock();overlay.user.IsIconic.return_value=False
  overlay.window=Mock();overlay.canvas=Mock();overlay.visible=True;overlay.hwnd=1;overlay.text=Mock()
  overlay.game_rect=Mock(return_value=(0,0,1920,1080));overlay.settings=Mock(current=[50,50])
  guide=n.CameraGuide();guide.update_angles((0,0,0),(1,1,1))
  overlay.render('Test',1,guide,True,True,quantum=True,travel_lines=['CLOSEST APPROACH IN 00:10'])
  overlay.canvas.create_line.assert_not_called()
  self.assertTrue(any('CLOSEST APPROACH' in str(call) for call in overlay.text.call_args_list))
if __name__=='__main__':unittest.main()
