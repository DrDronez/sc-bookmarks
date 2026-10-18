import sys,unittest

from mining_bookmarks import *
from bookmark_links import encode,decode
class MiningTests(unittest.TestCase):
 def rock(self):
  return {'name':'Example rock','system':'Nyx','category':'Mining','xyz_km':[1,2,3],
          'notes':'Come back with a bigger laser.\nNorth of the marker.',
          'mining':clean_mining({'mass':33686,'resistance':0,'instability':40,'scu':58.58,
           'composition':[{'ore':'Iron','percent':10.63,'quality':521},{'ore':'Iron','percent':35.52,'quality':325},
                          {'ore':'Inert Materials','percent':53.83,'quality':0}]})}
 def test_example(self):
  b=self.rock();r=b['mining']['composition']
  self.assertAlmostEqual(estimated_scu(58.58,r[0]),6.227054)
  self.assertAlmostEqual(estimated_scu(58.58,r[1]),20.807616)
  self.assertTrue(matches(b,'Iron',500));self.assertFalse(matches(b,'Iron',522))
  self.assertEqual(len(r),3)
 def test_blank_and_zero(self):
  self.assertIsNone(estimated_scu(58,{'percent':10,'quality':None}))
  self.assertIsNone(estimated_scu(58,{'percent':None,'quality':500}))
  self.assertIsNone(estimated_scu(None,{'percent':10,'quality':500}))
  self.assertEqual(estimated_scu(58,{'percent':0,'quality':0}),0)
 def test_same_row_match(self):
  b=self.rock();b['mining']['composition']=[{'ore':'Iron','quality':325,'percent':10},{'ore':'Gold','quality':900,'percent':20}]
  self.assertFalse(matches(b,'Iron',500));self.assertTrue(matches(b,'Gold',500))
 def test_sort(self):
  a=self.rock();b=self.rock();b['mining']['composition'][0]['quality']=700
  self.assertIs(filter_mining([a,b],'Iron',500,'Ore quality')[0],b)
 def test_share(self):
  b=self.rock();r=decode(encode(b))
  self.assertEqual(r['notes'],b['notes']);self.assertEqual(r['mining'],b['mining'])
  self.assertIn('v=2',encode(b))
 def test_validation(self):
  for data in [{'scu':float('nan')},{'resistance':101},{'composition':[{'ore':'Iron','percent':-1}]},{'composition':[{'ore':'Iron','quality':float('inf')}]}]:
   with self.assertRaises(ValueError):clean_mining(data)
if __name__=='__main__':unittest.main()
