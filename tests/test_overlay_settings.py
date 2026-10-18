import sys, tempfile, unittest
from pathlib import Path

from overlay_settings import Settings, placement, position

class OverlaySettingsTests(unittest.TestCase):
    def test_resolutions_and_monitor_origins(self):
        for w,h in [(1280,720),(1366,768),(1600,900),(1920,1080),(1920,1200),(2560,1440),(2560,1080),(3440,1440),(3840,1600),(3840,2160),(5120,1440),(5120,2160),(7680,2160)]:
            for left,top in [(0,0),(-3840,0),(1920,-1080)]:
                for horizontal in (0,25,50,75,100):
                    for vertical in (0,25,50,75,100):
                        x,y,ow,oh=placement((left,top,w,h),(horizontal,vertical))
                        self.assertTrue(left<=x<=left+w-ow)
                        self.assertTrue(top<=y<=top+h-oh)
                x,y,ow,oh=placement((left,top,w,h),(50,50))
                self.assertLessEqual(abs(x+ow/2-(left+w/2)),.5)
                self.assertLessEqual(abs(y+oh/2-(top+h/2)),.5)
    def test_persistence_and_default(self):
        with tempfile.TemporaryDirectory() as directory:
            path=Path(directory)/'settings.json';s=Settings(path)
            self.assertEqual(s.current,[50,50])
            s.save([0,100],{'Prospector':[0,100],'船':[80,15]})
            loaded=Settings(path)
            self.assertEqual(loaded.current,[0,100]);self.assertEqual(loaded.presets,s.presets)
            loaded.save([50,50],{'船':[80,15]})
            self.assertNotIn('Prospector',Settings(path).presets)
            path.write_text('{broken');self.assertEqual(Settings(path).current,[50,50])
    def test_invalid_values(self):
        for value in ([float('nan'),50],[0,float('inf')],[-1,50],[50,101],[],None):
            with self.assertRaises((ValueError,TypeError)):position(value)

if __name__=='__main__':unittest.main()
