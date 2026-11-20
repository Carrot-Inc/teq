"""classfile.py: a class file's name, access, parents, generic signature, source file and members (access, name,
descriptor, signature; a field's ConstantValue as `I:7`, `J:7`, `F:`/`D:` its bits in hex, `S:` its UTF-8 in hex),
read from its bytes (JVMS 4): the oracle's side of the descriptor fence."""
import struct

ACC_SYNTHETIC, ACC_BRIDGE = 0x1000, 0x0040


def _u2(b, i):
    return struct.unpack_from('>H', b, i)[0], i + 2


def _u4(b, i):
    return struct.unpack_from('>I', b, i)[0], i + 4


def _mutf8(raw):
    """A class file's modified UTF-8 (JVMS 4.4.7): NUL as C0 80, a supplementary character as its two surrogates each
    in three bytes; a lone surrogate kept as one."""
    units, i = [], 0
    while i < len(raw):
        c = raw[i]
        if c < 0x80:
            units.append(c); i += 1
        elif c >> 5 == 0x6:
            units.append(((c & 0x1f) << 6) | (raw[i + 1] & 0x3f)); i += 2
        else:
            units.append(((c & 0x0f) << 12) | ((raw[i + 1] & 0x3f) << 6) | (raw[i + 2] & 0x3f)); i += 3
    out, j = [], 0
    while j < len(units):
        u = units[j]
        if 0xd800 <= u < 0xdc00 and j + 1 < len(units) and 0xdc00 <= units[j + 1] < 0xe000:
            out.append(chr(0x10000 + ((u - 0xd800) << 10) + (units[j + 1] - 0xdc00))); j += 2
        else:
            out.append(chr(u)); j += 1
    return ''.join(out)


def read(data):
    """{'name', 'access', 'super', 'interfaces', 'signature', 'fields': [(access, name, desc, sig)], 'methods': [...]}"""
    b = data
    if b[:4] != b'\xca\xfe\xba\xbe':
        raise ValueError('not a class file')
    i = 8
    n, i = _u2(b, i)
    cp = [None] * n
    k = 1
    while k < n:
        tag = b[i]; i += 1
        if tag == 1:
            ln, i = _u2(b, i)
            cp[k] = ('utf8', _mutf8(b[i:i + ln])); i += ln
        elif tag == 3:
            cp[k] = ('const', 'I:%d' % struct.unpack_from('>i', b, i)[0]); i += 4
        elif tag == 4:
            cp[k] = ('const', 'F:%08x' % struct.unpack_from('>I', b, i)[0]); i += 4
        elif tag == 5:
            cp[k] = ('const', 'J:%d' % struct.unpack_from('>q', b, i)[0]); i += 8; k += 1
        elif tag == 6:
            cp[k] = ('const', 'D:%016x' % struct.unpack_from('>Q', b, i)[0]); i += 8; k += 1
        elif tag == 7:
            idx, i = _u2(b, i); cp[k] = ('class', idx)
        elif tag == 8:
            idx, i = _u2(b, i); cp[k] = ('string', idx)
        elif tag in (16, 19, 20):
            idx, i = _u2(b, i); cp[k] = ('other', idx)
        elif tag in (9, 10, 11, 12, 17, 18):
            i += 4
        elif tag == 15:
            i += 3
        else:
            raise ValueError(f'constant pool tag {tag}')
        k += 1
    utf = lambda j: cp[j][1]
    cls = lambda j: utf(cp[j][1]) if j else None
    access, i = _u2(b, i)
    this, i = _u2(b, i)
    sup, i = _u2(b, i)
    ni, i = _u2(b, i)
    ifaces = []
    for _ in range(ni):
        j, i = _u2(b, i); ifaces.append(cls(j))

    def constant(j):
        e = cp[j]
        if e[0] == 'const':
            return e[1]
        if e[0] == 'string':
            return 'S:' + utf(e[1]).encode('utf-8', 'surrogatepass').hex()
        return None

    def attrs(i):
        found = {}
        na, i = _u2(b, i)
        for _ in range(na):
            an, i = _u2(b, i)
            ln, i = _u4(b, i)
            a = utf(an)
            if a in ('Signature', 'SourceFile'):
                found[a] = utf(struct.unpack_from('>H', b, i)[0])
            elif a == 'ConstantValue':
                found[a] = constant(struct.unpack_from('>H', b, i)[0])
            i += ln
        return found, i

    def members(i, kind):
        out, consts = [], {}
        nm, i = _u2(b, i)
        for _ in range(nm):
            acc, i = _u2(b, i)
            name, i = _u2(b, i)
            desc, i = _u2(b, i)
            a, i = attrs(i)
            out.append((acc, utf(name), utf(desc), a.get('Signature')))
            if 'ConstantValue' in a:
                consts[utf(name)] = a['ConstantValue']
        return out, consts, i

    fields, consts, i = members(i, 'field')
    methods, _, i = members(i, 'method')
    ca, i = attrs(i)
    return {'name': cls(this), 'access': access, 'super': cls(sup), 'interfaces': ifaces, 'signature': ca.get('Signature'),
            'fields': fields, 'methods': methods, 'constants': consts, 'source': ca.get('SourceFile')}
