"""Offline geometric QT route suggestions; no game access or input."""
import json
import math
from functools import lru_cache
from itertools import combinations
from pathlib import Path


def point(value):
    if not isinstance(value, (list, tuple)) or len(value) != 3:
        raise ValueError('Three coordinates required')
    result = tuple(float(v) for v in value)
    if not all(math.isfinite(v) for v in result):
        raise ValueError('Coordinates must be finite')
    return result


def closest(target, start, end, lower=0.0, upper=1.0):
    delta = tuple(b-a for a, b in zip(start, end))
    length2 = sum(v*v for v in delta)
    if length2 <= 1e-12:
        return None
    fraction = sum((p-a)*d for p, a, d in zip(target, start, delta))/length2
    fraction = max(lower, min(upper, fraction))
    position = tuple(a+fraction*d for a, d in zip(start, delta))
    return math.dist(target, position), fraction, position


def suggest(bookmark, markers):
    target = point(bookmark.get('xyz_km'))
    if bookmark.get('frame', 'root').lower() != 'root':
        raise ValueError('Route suggestions require Root coordinates')
    system = str(bookmark.get('system', '')).strip().casefold()
    candidates = [m for m in markers if m['system'].casefold() == system]
    containing = [m for m in candidates
                  if m.get('body_radius_km', 0) > 0
                  and m.get('roi_radius_km', 0) > 0
                  and math.dist(target, point(m['xyz_km'])) <= m['roi_radius_km']]
    if containing:
        # Prefer the smaller region if a moon and planet's regions overlap.
        body = min(containing, key=lambda m: (m['roi_radius_km'],
                   math.dist(target, point(m['xyz_km'])), m['name']))
        return dict(celestial=body)
    best = None
    for a, b in combinations(candidates, 2):
        start, end = point(a['xyz_km']), point(b['xyz_km'])
        length = math.dist(start, end)
        # Do not promise a point inside the destination's QT arrival envelope.
        ra, rb = a['arrival_km'], b['arrival_km']
        if length <= ra+rb:
            continue
        low, high = ra/length, 1-rb/length
        result = closest(target, start, end, low, high)
        distance, fraction, position = result
        segment_start = tuple(x+low*(y-x) for x, y in zip(start, end))
        segment_end = tuple(x+high*(y-x) for x, y in zip(start, end))
        blocked = False
        for body in candidates:
            radius = body.get('body_radius_km', 0)
            if radius > 0:
                hit = closest(point(body['xyz_km']), segment_start, segment_end)
                if hit and hit[0] < radius:
                    blocked = True
                    break
        if blocked:
            continue
        error = max(a.get('position_error_km', 0), b.get('position_error_km', 0))
        item = dict(a=a, b=b, distance_km=distance, fraction=fraction,
                    position=position, allowance_km=error)
        # Rank nominal geometry, never an infinite extension of the route.
        if best is None or (distance, error, a['name'], b['name']) < (
                best['distance_km'], best['allowance_km'], best['a']['name'], best['b']['name']):
            best = item
    return best


def route_text(bookmark, path=None):
    if bookmark is None:
        return 'Select a bookmark to see a suggested QT route.'
    try:
        path = path or Path(__file__).with_name('route-data') / 'quantum-markers.json'
        data = json.loads(Path(path).read_text(encoding='utf-8'))
        result = suggest(bookmark, data['markers'])
    except (OSError, ValueError, TypeError, KeyError) as exc:
        return 'QT route unavailable: '+str(exc)
    if result is None:
        return 'No usable marker pair for this bookmark’s system and coordinates.'
    if 'celestial' in result:
        return result['celestial']['name']
    r = result
    return (f"{r['a']['name']} ↔ {r['b']['name']}\n"
            f"Approx. non-QT flight: {r['distance_km']:,.1f} km\n"
            f"Closest point: {r['fraction']:.1%} from {r['a']['name']}\n"
            f"Marker-position allowance: up to {r['allowance_km']:,.1f} km\n"
            'Best modeled pair in the offline catalog. Either direction; actual QT exits and route availability vary.')


@lru_cache(maxsize=512)
def _route_line(system, coordinates, frame):
    """Cache by destination values so sorting/filtering never repeats geometry."""
    text = route_text(dict(system=system, xyz_km=coordinates, frame=frame))
    lines = text.splitlines()
    if len(lines) == 1 and not text.startswith(('QT route unavailable:', 'No usable marker pair')):
        return 'QT: '+text
    if len(lines) < 4:
        return 'QT: unavailable'
    pair = lines[0]
    return f'QT: {pair}'


def route_line(bookmark):
    try:
        return _route_line(str(bookmark.get('system', '')), point(bookmark.get('xyz_km')),
                           str(bookmark.get('frame', 'root')))
    except (ValueError, TypeError):
        return 'QT: unavailable (invalid coordinates)'
