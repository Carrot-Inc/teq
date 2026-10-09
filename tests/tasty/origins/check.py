"""The checks of teq's TeqOrigins section on the pickles
tests/tasty.sh gives it.

  check.py roundtrip <teq> <source> <pickle>...  every pickle's section names the source's key
      and a token, and its definitions are the Positions section's entries with a point, each
      the byte offset in the source's text of the UTF-16 point the entry gives
  check.py uuid <pickle>...  the header's UUID is TastyPickler's hash of the name table and the
      sections, the ASTs and the name table in its low half and every other section in its high
  check.py patch <pickle> <out> version|variant|frame  a copy with the section's version made 2,
      its variant's length past the section (malformed within valid framing), or the section's
      own length past the file (a broken frame)
"""
import re
import subprocess
import sys


def nat(b, i):
    x = 0
    while True:
        d = b[i]
        i += 1
        x = x * 128 + (d & 0x7F)
        if d & 0x80:
            return x, i


def wnat(x):
    out = [x & 0x7F | 0x80]
    x >>= 7
    while x:
        out.append(x & 0x7F)
        x >>= 7
    return bytes(reversed(out))


def layout(b):
    """The header's end, the name table's (start, end), and per section (name, start, end) of
    its payload with the start of its frame."""
    i = 4
    for _ in range(3):
        _, i = nat(b, i)
    n, i = nat(b, i)
    i += n
    uuid_at = i
    i += 16
    tlen, j = nat(b, i)
    names = (j, j + tlen)
    names_list = []
    k = j
    while k < j + tlen:
        tag = b[k]
        k += 1
        ln, k = nat(b, k)
        names_list.append(b[k:k + ln] if tag == 1 else None)
        k += ln
    sections = []
    i = j + tlen
    while i < len(b):
        frame = i
        ref, i = nat(b, i)
        ln, i = nat(b, i)
        sections.append((names_list[ref], i, i + ln, frame))
        i += ln
    return uuid_at, names, sections


def pjw(data):
    h = 0
    for c in data:
        h = ((h << 8) + c) & 0xFFFFFFFFFFFFFFFF
        high = h & 0xFF00000000000000
        h ^= high >> 48
        h &= ~high & 0xFFFFFFFFFFFFFFFF
    return h


def uuid(paths):
    bad = 0
    for p in paths:
        b = open(p, 'rb').read()
        at, (ns, ne), sections = layout(b)
        low = pjw(b[ns:ne]) ^ pjw(b[sections[0][1]:sections[0][2]])
        high = 0
        for _, s, e, _ in sections[1:]:
            high ^= pjw(b[s:e])
        want = low.to_bytes(8, 'big') + high.to_bytes(8, 'big')
        if b[at:at + 16] != want:
            print(f'FAIL uuid {p}: header {b[at:at + 16].hex()}, the sections hash to {want.hex()}')
            bad += 1
    return bad


def units_to_bytes(text):
    """Per UTF-16 offset of the text its byte offset."""
    out = []
    at = 0
    for ch in text:
        n = len(ch.encode('utf-8'))
        units = 2 if ord(ch) > 0xFFFF else 1
        for _ in range(units):
            out.append(at)
        at += n
    out.append(at)
    return out


def roundtrip(teq, source, pickles):
    text = open(source, encoding='utf-8').read()
    to_bytes = units_to_bytes(text)
    key = source.rsplit('/', 1)[-1]
    bad = 0
    for p in pickles:
        dump = subprocess.run([teq, 'tasty', '--trees', p], capture_output=True, text=True).stdout
        points = {}
        # The definitions' addresses: the trees whose points the section's kind 1 records, apart
        # from the expressions' points.
        defs = set()
        section = None
        for line in dump.splitlines():
            if line.startswith('Trees'):
                section = 'trees'
            elif section == 'trees' and not line.startswith('Positions:'):
                m = re.match(r'\s*(\d+):\s*(VALDEF|DEFDEF|TYPEDEF|PARAM|TYPEPARAM)\b', line)
                if m:
                    defs.add(int(m.group(1)))
                continue
            if line.startswith('Positions:'):
                section = 'positions'
            elif line.startswith('Attributes:'):
                section = None
            elif line.startswith('TeqOrigins:'):
                section = 'origins'
                m = re.match(r'TeqOrigins: version 1, key "(.*)", token (\d+)$', line)
                if not m or m.group(1) != key:
                    print(f'FAIL roundtrip {p}: {line!r}, not the key {key!r}')
                    bad += 1
                origins = {}
            elif section == 'positions':
                m = re.match(r'\s+(\d+): (\d+) \.\. (\d+) point (\d+)$', line)
                if m:
                    points[int(m.group(1))] = int(m.group(4))
            elif section == 'origins':
                m = re.match(r'\s+(\d+): byte (\d+)$', line)
                if m:
                    origins[int(m.group(1))] = int(m.group(2))
        if section != 'origins' and 'TeqOrigins:' not in dump:
            print(f'FAIL roundtrip {p}: no TeqOrigins section')
            bad += 1
            continue
        want = {a: to_bytes[u] for a, u in points.items() if a in defs}
        if want != origins or not origins:
            print(f'FAIL roundtrip {p}: definitions {sorted(origins.items())}, the points give {sorted(want.items())}')
            bad += 1
    return bad


def patch(path, out, how):
    b = bytearray(open(path, 'rb').read())
    _, _, sections = layout(bytes(b))
    name, s, e, frame = sections[-1]
    assert name == b'TeqOrigins', name
    if how == 'version':
        b[s] = 0x82
    elif how == 'variant':
        # The payload: version, key, token, then the variant's kind, version and length.
        i = s
        _, i = nat(b, i)
        n, i = nat(b, i)
        i += n
        _, i = nat(b, i)
        _, i = nat(b, i)
        _, i = nat(b, i)
        _, j = nat(b, i)
        assert j == i + 1, 'a one-byte length'
        b[i] = 0x80 | 0x7F
    elif how == 'frame':
        _, i = nat(b, frame)
        _, j = nat(b, i)
        b = b[:i] + wnat(e - s + 1000) + b[j:]
    open(out, 'wb').write(bytes(b))


if __name__ == '__main__':
    what = sys.argv[1]
    if what == 'roundtrip':
        sys.exit(1 if roundtrip(sys.argv[2], sys.argv[3], sys.argv[4:]) else 0)
    if what == 'uuid':
        sys.exit(1 if uuid(sys.argv[2:]) else 0)
    if what == 'patch':
        patch(sys.argv[2], sys.argv[3], sys.argv[4])
