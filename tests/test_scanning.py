import sys,unittest,csv
from pathlib import Path

from scanning_reference import load_charts,filter_rows,parse_rs,CHARTS
class ScanningTests(unittest.TestCase):
 def test_totals(self):
  for chart,data in load_charts().items():
   if chart!='Salvage':
    for row in data['rows']:self.assertEqual(list(row[1:]),[row[1]*n for n in range(1,7)])
 def test_ambiguity(self):
  charts=load_charts();space=charts['Mining — Space']['rows'];surface=charts['Mining — Planetary Surface']['rows']
  self.assertEqual({r[0] for r in filter_rows('Mining — Space',space,rs=19200)},{'Savrilium','Aslarite'})
  self.assertEqual({r[0] for r in filter_rows('Mining — Planetary Surface',surface,rs=12000)},{'Hand mining','ROC mining'})
  self.assertIn('Redeemer',filter_rows('Salvage',charts['Salvage']['rows'],rs=2000)[0][1])
 def test_input(self):
  self.assertEqual(parse_rs('18,000'),18000);self.assertIsNone(parse_rs(''))
  for value in ['x','-1','0','18.5']:
   with self.assertRaises(ValueError):parse_rs(value)
if __name__=='__main__':unittest.main()
