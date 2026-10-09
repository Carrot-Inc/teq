# tests/support/identity-root.py <root> <dir>: the checkout's absolute path, which a macro such as sourcecode's File
# writes into a program's output, replaced by `<checkout>` in every file under <dir>, so that two machines'
# outputs of one program compare equal (tests/support/identity-across.sh). A text file is rewritten by a plain
# replacement; a class file by rewriting the CONSTANT_Utf8 entries of its constant pool that hold the path, their
# lengths recomputed, the rest of the file byte for byte as it was.
import os
import sys

root, where = sys.argv[1].encode(), sys.argv[2]
if len(root) < 2:
    sys.exit("identity-root: no checkout path given")
placeholder = b"<checkout>"


def utf8_entries(data):
    """The constant pool's entries as (offset of the tag, tag, end offset)."""
    count = int.from_bytes(data[8:10], "big")
    at, i, out = 10, 1, []
    while i < count:
        tag = data[at]
        if tag == 1:
            n = int.from_bytes(data[at + 1:at + 3], "big")
            out.append((at, tag, at + 3 + n))
            at += 3 + n
        elif tag in (3, 4):
            at += 5
        elif tag in (5, 6):
            at += 9
            i += 1
        elif tag in (7, 8, 16, 19, 20):
            at += 3
        elif tag in (9, 10, 11, 12, 17, 18):
            at += 5
        elif tag == 15:
            at += 4
        else:
            raise ValueError("constant pool tag %d" % tag)
        i += 1
    return out, at


def rewrite_class(data):
    entries, end = utf8_entries(data)
    out, last = bytearray(), 0
    for at, tag, stop in entries:
        text = data[at + 3:stop]
        if root in text:
            new = text.replace(root, placeholder)
            out += data[last:at] + b"\x01" + len(new).to_bytes(2, "big") + new
            last = stop
    if last == 0:
        return data
    out += data[last:]
    return bytes(out)


for dirpath, _, names in os.walk(where):
    for name in names:
        path = os.path.join(dirpath, name)
        with open(path, "rb") as f:
            data = f.read()
        if root not in data:
            continue
        new = rewrite_class(data) if name.endswith(".class") and data[:4] == b"\xca\xfe\xba\xbe" else data.replace(root, placeholder)
        if new != data:
            with open(path, "wb") as f:
                f.write(new)
