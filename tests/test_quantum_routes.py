import json
import math
from pathlib import Path
import tempfile
import unittest
from quantum_routes import closest, suggest, route_text


def marker(name, xyz, system='Nyx', arrival=0, radius=0, error=0):
    return dict(name=name, xyz_km=xyz, system=system, arrival_km=arrival,
                body_radius_km=radius, position_error_km=error)


class Routes(unittest.TestCase):
    def test_roi_boundary_and_outside(self):
        body=marker('Planet',[0,0,0],radius=1,arrival=2)
        body['roi_radius_km']=5
        other=marker('Station',[100,0,0])
        inside=suggest(dict(system='Nyx',xyz_km=[5,0,0]),[body,other])
        self.assertEqual(inside['celestial']['name'],'Planet')
        outside=suggest(dict(system='Nyx',xyz_km=[5.001,0,0]),[body,other])
        self.assertNotIn('celestial',outside)

    def test_overlapping_roi_prefers_moon_and_respects_system(self):
        planet=marker('Planet',[0,0,0],radius=1)
        planet['roi_radius_km']=100
        moon=marker('Moon',[10,0,0],radius=1)
        moon['roi_radius_km']=5
        foreign=marker('Foreign',[10,0,0],system='Stanton',radius=1)
        foreign['roi_radius_km']=2
        result=suggest(dict(system='Nyx',xyz_km=[10,0,0]),[planet,moon,foreign])
        self.assertEqual(result['celestial']['name'],'Moon')

    def test_compact_celestial_row(self):
        from quantum_routes import route_line
        self.assertEqual(route_line(dict(system='Stanton',xyz_km=[12850457.093,0,0])),
                         'QT: Hurston')

    def test_perpendicular_distance(self):
        r=suggest(dict(system='Nyx',xyz_km=[5,3,0]),[
            marker('A',[0,0,0]),marker('B',[10,0,0])])
        self.assertEqual(r['distance_km'],3)
        self.assertEqual(r['fraction'],.5)

    def test_segment_not_infinite_line_and_arrival_limits(self):
        r=suggest(dict(system='Nyx',xyz_km=[20,0,0]),[
            marker('A',[0,0,0],arrival=1),marker('B',[10,0,0],arrival=2)])
        self.assertEqual(r['distance_km'],12)
        self.assertEqual(r['fraction'],.8)

    def test_system_isolation(self):
        self.assertIsNone(suggest(dict(system='Nyx',xyz_km=[0,0,0]),[
            marker('A',[1,0,0]),marker('B',[-1,0,0],system='Stanton')]))

    def test_duplicate_positions(self):
        self.assertIsNone(suggest(dict(system='Nyx',xyz_km=[0,0,0]),[
            marker('A',[1,0,0]),marker('B',[1,0,0])]))

    def test_obstruction(self):
        markers=[marker('A',[-10,0,0]),marker('B',[10,0,0]),
                 marker('Body',[0,0,0],arrival=3,radius=2)]
        r=suggest(dict(system='Nyx',xyz_km=[0,1,0]),markers)
        self.assertNotEqual({r['a']['name'],r['b']['name']},{'A','B'})

    def test_invalid_coordinates_and_frame(self):
        for target in ([1,2], [1,math.nan,0]):
            with self.assertRaises(ValueError):suggest(dict(system='Nyx',xyz_km=target),[])
        with self.assertRaises(ValueError):
            suggest(dict(system='Nyx',xyz_km=[1,2,3],frame='local'),[])

    def test_allowance_and_best_pair(self):
        r=suggest(dict(system='Nyx',xyz_km=[5,3,0]),[
            marker('A',[0,0,0],error=2),marker('B',[10,0,0],error=4),
            marker('C',[10,6,0])])
        self.assertEqual({r['a']['name'],r['b']['name']},{'A','C'})
        self.assertAlmostEqual(r['distance_km'],0)
        self.assertEqual(r['allowance_km'],2)

    def test_catalog_and_missing_data(self):
        self.assertIn('unavailable',route_text(dict(system='Nyx',xyz_km=[1,2,3]),'nonexistent.json'))
        data=json.loads((Path(__file__).parents[1]/'route-data/quantum-markers.json').read_text())
        self.assertEqual(len(data['markers']),70)
        for system in ('Stanton','Pyro','Nyx'):
            self.assertIsNotNone(suggest(dict(system=system,xyz_km=[0,0,0]),data['markers']))


if __name__=='__main__':unittest.main()
