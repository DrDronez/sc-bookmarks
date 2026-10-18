import sys,unittest,math

from bookmark_links import encode,decode,decode_many
class LinkTests(unittest.TestCase):
 def test_roundtrip(self):
  b={'name':'Cache & cargo / α + #1','system':'Nyx','xyz_km':[123.4567,-89.1234,.0001],'shard_id':'example_shard','utc':'2026-10-18T12:00:00Z'}
  r=decode(encode(b))
  for k in b:self.assertEqual(r[k],b[k])
 def test_old_bookmarks_and_bulk(self):
  b={'name':'Old','system':'Nyx','xyz_km':[1,2,3]}
  self.assertIsNone(decode(encode(b))['shard_id'])
  self.assertEqual(len(decode_many(encode(b)+'\n\n'+encode(b))),2)
 def test_invalid(self):
  link=encode({'name':'Test','system':'Stanton','xyz_km':[1,2,3]})
  for bad in [link.replace('v=1','v=99'),link+'&x=2',link.replace('units=km','units=m'),link.replace('x=1.0','x=nan'),link.replace('frame=root','frame=ship'), 'https://example.com']:
   with self.assertRaises(ValueError):decode(bad)
 def test_batch_failure_is_not_partial(self):
  link=encode({'name':'Test','system':'Nyx','xyz_km':[1,2,3]})
  with self.assertRaises(ValueError):decode_many(link+'\ninvalid')
if __name__=='__main__':unittest.main()

