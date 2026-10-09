"""A copy of a jar whose TASTy files lack the named sections, their frames removed and their names
left in the name table: what a pickle written without them reads as.

usage: strip.py <jar> <out jar> <section>..."""
import sys
import zipfile


def nat(b, i):
    x = 0
    while True:
        d = b[i]
        i += 1
        x = x * 128 + (d & 0x7F)
        if d & 0x80:
            return x, i


def strip(b, drop):
    i = 4
    for _ in range(3):
        _, i = nat(b, i)
    n, i = nat(b, i)
    i += n + 16
    tlen, j = nat(b, i)
    names = []
    k = j
    while k < j + tlen:
        tag = b[k]
        k += 1
        ln, k = nat(b, k)
        names.append(b[k:k + ln] if tag == 1 else None)
        k += ln
    out = bytearray(b[:j + tlen])
    i = j + tlen
    while i < len(b):
        frame = i
        ref, i = nat(b, i)
        ln, i = nat(b, i)
        if names[ref] not in drop:
            out += b[frame:i + ln]
        i += ln
    return bytes(out)


src, dst, sections = sys.argv[1], sys.argv[2], {s.encode() for s in sys.argv[3:]}
with zipfile.ZipFile(src) as zi, zipfile.ZipFile(dst, 'w', zipfile.ZIP_DEFLATED) as zo:
    for e in zi.infolist():
        data = zi.read(e)
        zo.writestr(e.filename, strip(data, sections) if e.filename.endswith('.tasty') else data)
