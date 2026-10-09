#!/usr/bin/env python3
"""tests/support/classdiff.py <reference dir> <dir> [--mode NAME]: the class files of two builds (trees of .class files,
the same relative paths on both sides) compared as parsed structures. A class file is read whole: its constant pool
resolved, so that every reference is compared by what it names and not by its index; its fields, methods and
attributes, those of the class, of each member and of each method's code, with their contents kept and their order
made canonical (by name); the code of a method instruction by instruction, each operand that names a constant
resolved, and the stack map frames and the tables likewise. An attribute of a kind the reader knows is compared
resolved, any other byte for byte. Prints one line per removed or added member, `<mode>\\t<removed|added>\\t<owner>\\t
<name><descriptor>\\t<access>`, a line per other difference, and a summary; exits 1 when a class file differs in
anything but abstract methods removed, or when one cannot be read whole.
tests/support/classdiff.py --self-test: the comparison on class files javac writes, when a javac is on the path."""
import os
import struct
import sys

ACC_ABSTRACT = 0x0400


class Reader:
    def __init__(self, data):
        self.b, self.at = data, 0

    def take(self, n):
        v = self.b[self.at:self.at + n]
        if len(v) != n:
            raise ValueError("truncated")
        self.at += n
        return v

    def u1(self):
        return self.take(1)[0]

    def u2(self):
        return struct.unpack(">H", self.take(2))[0]

    def s2(self):
        return struct.unpack(">h", self.take(2))[0]

    def u4(self):
        return struct.unpack(">I", self.take(4))[0]

    def s4(self):
        return struct.unpack(">i", self.take(4))[0]

    def done(self):
        return self.at == len(self.b)


# Operand lengths of the instructions whose operands name no constant.
PLAIN = {0x10: 1, 0x11: 2, 0x15: 1, 0x16: 1, 0x17: 1, 0x18: 1, 0x19: 1, 0x36: 1, 0x37: 1, 0x38: 1, 0x39: 1, 0x3A: 1,
         0x84: 2, 0xA9: 1, 0xBC: 1, 0xC6: 2, 0xC7: 2, 0xC8: 4, 0xC9: 4, **{op: 2 for op in range(0x99, 0xA9)}}
# The instructions whose operand is a two-byte constant index: ldc_w, ldc2_w, the field instructions, invokevirtual,
# invokespecial, invokestatic, new, anewarray, checkcast, instanceof.
CONSTANT = {0x13, 0x14, 0xB2, 0xB3, 0xB4, 0xB5, 0xB6, 0xB7, 0xB8, 0xBB, 0xBD, 0xC0, 0xC1}


class ClassFile:
    """A class file with its constant pool resolved: `resolve(i)` is what entry i names, a tuple of plain values."""

    def __init__(self, data):
        r = Reader(data)
        if r.u4() != 0xCAFEBABE:
            raise ValueError("no class file")
        self.version = (r.u2(), r.u2())
        count = r.u2()
        self.pool = [None] * count
        i = 1
        while i < count:
            tag = r.u1()
            if tag == 1:
                self.pool[i] = ("utf8", r.take(r.u2()).decode("utf-8", "surrogateescape"))
            elif tag in (3, 4):
                self.pool[i] = ("int" if tag == 3 else "float", r.take(4).hex())
            elif tag in (5, 6):
                self.pool[i] = ("long" if tag == 5 else "double", r.take(8).hex())
                i += 1
            elif tag in (7, 8, 16, 19, 20):
                self.pool[i] = ({7: "class", 8: "string", 16: "methodtype", 19: "module", 20: "package"}[tag], r.u2())
            elif tag in (9, 10, 11, 12, 17, 18):
                self.pool[i] = ({9: "field", 10: "method", 11: "imethod", 12: "nat", 17: "dynamic", 18: "indy"}[tag], r.u2(), r.u2())
            elif tag == 15:
                self.pool[i] = ("handle", r.u1(), r.u2())
            else:
                raise ValueError(f"constant pool tag {tag}")
            i += 1
        self.bootstraps = []
        self.access = r.u2()
        self.this = self.resolve(r.u2())
        sup = r.u2()
        self.super = self.resolve(sup) if sup else None
        self.interfaces = tuple(self.resolve(r.u2()) for _ in range(r.u2()))
        fields = [self.member(r) for _ in range(r.u2())]
        methods = [self.member(r) for _ in range(r.u2())]
        raw = [(self.utf8(r.u2()), r.take(r.u4())) for _ in range(r.u2())]
        if not r.done():
            raise ValueError("bytes after the class")
        # The bootstrap methods first: the code's invokedynamic and the dynamic constants name them.
        for name, data in raw:
            if name == "BootstrapMethods":
                br = Reader(data)
                self.bootstraps = [(self.resolve(br.u2()), tuple(self.resolve(br.u2()) for _ in range(br.u2()))) for _ in range(br.u2())]
                if not br.done():
                    raise ValueError(f"BootstrapMethods: {len(data) - br.at} bytes left unread")
        # The fields by name and descriptor, as the attributes are by name: their order in the file means nothing.
        self.fields = tuple(sorted((self.finish(m) for m in fields), key=lambda f: (f[1], f[2])))
        self.methods = [self.finish(m) for m in methods]
        self.attributes = self.attrs(raw)

    def utf8(self, i):
        e = self.pool[i]
        if e is None or e[0] != "utf8":
            raise ValueError(f"entry {i} is no Utf8")
        return e[1]

    def resolve(self, i):
        e = self.pool[i] if 0 < i < len(self.pool) else None
        if e is None:
            raise ValueError(f"no constant {i}")
        kind = e[0]
        if kind in ("utf8", "int", "float", "long", "double"):
            return e
        if kind in ("class", "string", "methodtype", "module", "package"):
            return (kind, self.utf8(e[1]))
        if kind in ("field", "method", "imethod"):
            return (kind, self.resolve(e[1]), self.resolve(e[2]))
        if kind == "nat":
            return (kind, self.utf8(e[1]), self.utf8(e[2]))
        if kind in ("dynamic", "indy"):
            if e[1] >= len(self.bootstraps):
                raise ValueError(f"no bootstrap method {e[1]}")
            return (kind, self.bootstraps[e[1]], self.resolve(e[2]))
        if kind == "handle":
            return (kind, e[1], self.resolve(e[2]))
        raise ValueError(kind)

    def opt(self, i):
        return self.resolve(i) if i else None

    def member(self, r):
        access, name, desc = r.u2(), self.utf8(r.u2()), self.utf8(r.u2())
        return (access, name, desc, [(self.utf8(r.u2()), r.take(r.u4())) for _ in range(r.u2())])

    def finish(self, m):
        return (m[0], m[1], m[2], self.attrs(m[3]))

    def attrs(self, raw):
        return tuple(sorted((name, self.attribute(name, data)) for name, data in raw))

    def attribute(self, name, data):
        r = Reader(data)
        if name == "Code":
            out = self.code(r)
        elif name in ("SourceFile", "Signature"):
            out = self.utf8(r.u2())
        elif name in ("ConstantValue", "NestHost", "ModuleMainClass"):
            out = self.resolve(r.u2())
        elif name in ("Exceptions", "NestMembers", "PermittedSubclasses"):
            out = tuple(self.resolve(r.u2()) for _ in range(r.u2()))
        elif name == "InnerClasses":
            out = tuple((self.opt(r.u2()), self.opt(r.u2()), self.opt(r.u2()), r.u2()) for _ in range(r.u2()))
        elif name == "EnclosingMethod":
            out = (self.resolve(r.u2()), self.opt(r.u2()))
        elif name == "BootstrapMethods":
            out = tuple(self.bootstraps)
            r.at = len(data)
        elif name == "LineNumberTable":
            out = tuple((r.u2(), r.u2()) for _ in range(r.u2()))
        elif name in ("LocalVariableTable", "LocalVariableTypeTable"):
            out = tuple((r.u2(), r.u2(), self.utf8(r.u2()), self.utf8(r.u2()), r.u2()) for _ in range(r.u2()))
        elif name == "StackMapTable":
            out = self.frames(r)
        elif name == "MethodParameters":
            out = tuple((self.opt(r.u2()), r.u2()) for _ in range(r.u1()))
        elif name in ("RuntimeVisibleAnnotations", "RuntimeInvisibleAnnotations"):
            out = tuple(self.annotation(r) for _ in range(r.u2()))
        elif name in ("RuntimeVisibleParameterAnnotations", "RuntimeInvisibleParameterAnnotations"):
            out = tuple(tuple(self.annotation(r) for _ in range(r.u2())) for _ in range(r.u1()))
        elif name == "AnnotationDefault":
            out = self.element(r)
        elif name in ("Synthetic", "Deprecated"):
            out = ()
        else:
            # A kind no reader here knows (the TASTY attribute, a Scala signature): its bytes, which an index into
            # the constant pool would make differ, so that nothing unread passes.
            return ("bytes", data.hex())
        if not r.done():
            raise ValueError(f"{name}: {len(data) - r.at} bytes left unread")
        return out

    def annotation(self, r):
        kind = self.utf8(r.u2())
        return (kind, tuple((self.utf8(r.u2()), self.element(r)) for _ in range(r.u2())))

    def element(self, r):
        tag = chr(r.u1())
        if tag in "BCDFIJSZs":
            return (tag, self.resolve(r.u2()))
        if tag == "e":
            return (tag, self.utf8(r.u2()), self.utf8(r.u2()))
        if tag == "c":
            return (tag, self.utf8(r.u2()))
        if tag == "@":
            return (tag, self.annotation(r))
        if tag == "[":
            return (tag, tuple(self.element(r) for _ in range(r.u2())))
        raise ValueError(f"annotation element tag {tag}")

    def vtype(self, r):
        tag = r.u1()
        if tag == 7:
            return ("object", self.resolve(r.u2()))
        if tag == 8:
            return ("uninitialized", r.u2())
        if tag > 8:
            raise ValueError(f"verification type {tag}")
        return (tag,)

    def frames(self, r):
        out = []
        for _ in range(r.u2()):
            t = r.u1()
            if t < 64:
                out.append(("same", t))
            elif t < 128:
                out.append(("same_locals_1", t, self.vtype(r)))
            elif t == 247:
                out.append(("same_locals_1_extended", r.u2(), self.vtype(r)))
            elif 248 <= t <= 251:
                out.append(("chop_or_same_extended", t, r.u2()))
            elif 252 <= t <= 254:
                delta = r.u2()
                out.append(("append", t, delta, tuple(self.vtype(r) for _ in range(t - 251))))
            elif t == 255:
                delta = r.u2()
                locals_ = tuple(self.vtype(r) for _ in range(r.u2()))
                out.append(("full", delta, locals_, tuple(self.vtype(r) for _ in range(r.u2()))))
            else:
                raise ValueError(f"frame type {t}")
        return tuple(out)

    def code(self, r):
        max_stack, max_locals = r.u2(), r.u2()
        insns = self.instructions(r.take(r.u4()))
        table = tuple((r.u2(), r.u2(), r.u2(), self.opt(r.u2())) for _ in range(r.u2()))
        raw = [(self.utf8(r.u2()), r.take(r.u4())) for _ in range(r.u2())]
        return (max_stack, max_locals, insns, table, self.attrs(raw))

    def instructions(self, code):
        r = Reader(code)
        out = []
        while not r.done():
            pc = r.at
            op = r.u1()
            if op == 0x12:
                out.append((pc, op, self.resolve(r.u1())))
            elif op in CONSTANT:
                out.append((pc, op, self.resolve(r.u2())))
            elif op == 0xB9:
                out.append((pc, op, self.resolve(r.u2()), r.u1(), r.u1()))
            elif op == 0xBA:
                out.append((pc, op, self.resolve(r.u2()), r.u2()))
            elif op == 0xC5:
                out.append((pc, op, self.resolve(r.u2()), r.u1()))
            elif op == 0xAA:
                r.take((4 - r.at % 4) % 4)
                default, low, high = r.s4(), r.s4(), r.s4()
                out.append((pc, op, default, low, high, tuple(r.s4() for _ in range(high - low + 1))))
            elif op == 0xAB:
                r.take((4 - r.at % 4) % 4)
                default, n = r.s4(), r.s4()
                out.append((pc, op, default, tuple((r.s4(), r.s4()) for _ in range(n))))
            elif op == 0xC4:
                inner = r.u1()
                out.append((pc, op, inner, r.u2(), r.s2() if inner == 0x84 else None))
            elif op > 0xC9:
                raise ValueError(f"opcode {op:#x}")
            else:
                out.append((pc, op, r.take(PLAIN.get(op, 0)).hex()))
        return tuple(out)


def compare(ref, new, mode):
    """The comparison's lines and its counts: files, removed, added, other, unread."""
    lines = []
    files = removed = added = other = unread = 0
    for cur, _, names in os.walk(ref):
        for n in sorted(names):
            if not n.endswith(".class"):
                continue
            a = os.path.join(cur, n)
            rel = os.path.relpath(a, ref)
            b = os.path.join(new, rel)
            owner = rel[:-len(".class")]
            if not os.path.exists(b):
                lines.append(f"{mode}\tfile-removed\t{rel}")
                other += 1
                continue
            da, db = open(a, "rb").read(), open(b, "rb").read()
            if da == db:
                continue
            files += 1
            try:
                ca, cb = ClassFile(da), ClassFile(db)
            except Exception as e:
                lines.append(f"{mode}\tunread\t{owner}\t{e}")
                unread += 1
                continue
            differences = [what for what in ("version", "access", "this", "super", "interfaces", "fields", "attributes") if getattr(ca, what) != getattr(cb, what)]
            ma = {(m[1], m[2]): m for m in ca.methods}
            mb = {(m[1], m[2]): m for m in cb.methods}
            if len(ma) != len(ca.methods) or len(mb) != len(cb.methods):
                differences.append("a method declared twice")
            for key in sorted(set(ma) - set(mb)):
                acc = ma[key][0]
                lines.append(f"{mode}\tremoved\t{owner}\t{key[0]}{key[1]}\t{'abstract' if acc & ACC_ABSTRACT else hex(acc)}")
                removed += 1
                if not acc & ACC_ABSTRACT:
                    differences.append(f"{key[0]}{key[1]} removed")
            for key in sorted(set(mb) - set(ma)):
                lines.append(f"{mode}\tadded\t{owner}\t{key[0]}{key[1]}\t{hex(mb[key][0])}")
                added += 1
                differences.append(f"{key[0]}{key[1]} added")
            differences.extend(f"{key[0]}{key[1]} changed" for key in sorted(set(ma) & set(mb)) if ma[key] != mb[key])
            if differences:
                lines.append(f"{mode}\tother\t{owner}\t{', '.join(differences)[:300]}")
                other += 1
    for cur, _, names in os.walk(new):
        for n in sorted(names):
            rel = os.path.relpath(os.path.join(cur, n), new)
            if n.endswith(".class") and not os.path.exists(os.path.join(ref, rel)):
                lines.append(f"{mode}\tfile-added\t{rel}")
                other += 1
    return lines, (files, removed, added, other, unread)


def padded_attribute(data, wanted):
    """`data` with a zero byte after the content of the class's attribute `wanted`, its length grown to match."""
    r = Reader(data)
    r.take(8)
    count = r.u2()
    names = {}
    i = 1
    while i < count:
        tag = r.u1()
        if tag == 1:
            names[i] = r.take(r.u2()).decode("utf-8", "surrogateescape")
        elif tag in (5, 6):
            r.take(8)
            i += 1
        else:
            r.take({3: 4, 4: 4, 7: 2, 8: 2, 16: 2, 19: 2, 20: 2, 9: 4, 10: 4, 11: 4, 12: 4, 17: 4, 18: 4, 15: 3}[tag])
        i += 1
    r.take(6)
    r.take(2 * r.u2())
    for _ in range(2):
        for _ in range(r.u2()):
            r.take(6)
            for _ in range(r.u2()):
                r.take(2)
                r.take(r.u4())
    for _ in range(r.u2()):
        name = names[r.u2()]
        at = r.at
        length = r.u4()
        if name == wanted:
            end = at + 4 + length
            return data[:at] + struct.pack(">I", length + 1) + data[at + 4:end] + b"\0" + data[end:]
        r.take(length)
    raise ValueError(f"no attribute {wanted}")


def self_test():
    import shutil
    import subprocess
    import tempfile
    if not shutil.which("javac"):
        print("classdiff self-test: skipped, no javac")
        return 0
    base = ("abstract class C { public int a; public int b; abstract int gone(); abstract int kept(); int one() { return 1; }"
            " String s() { return \"#123\"; }\n int two() { return 2; }\n java.util.function.IntSupplier f() { return () -> 4; } }\n")
    variants = {
        "an abstract method removed": (base.replace(" abstract int gone();", ""), True),
        "fields declared the other way round": (base.replace("public int a; public int b;", "public int b; public int a;"), True),
        "a string changed": (base.replace("#123", "#456"), False),
        "a line moved": (base.replace("\n int two", "\n\n int two"), False),
        "a constant changed": (base.replace("return 2", "return 3"), False),
        "a concrete method removed": (base.replace(" int one() { return 1; }", ""), False),
    }
    failures = []
    count = len(variants) + 3
    with tempfile.TemporaryDirectory() as d:
        def build(where, text, flags):
            os.makedirs(where)
            open(os.path.join(where, "C.java"), "w").write(text)
            subprocess.run(["javac", "-d", where, *flags, os.path.join(where, "C.java")], check=True, capture_output=True)
            os.remove(os.path.join(where, "C.java"))
        build(os.path.join(d, "ref"), base, ["-g"])
        for i, (name, (text, passes)) in enumerate(variants.items()):
            where = os.path.join(d, f"v{i}")
            build(where, text, ["-g"])
            _, counts = compare(os.path.join(d, "ref"), where, "t")
            if (counts[3] == 0 and counts[4] == 0) != passes:
                failures.append(f"{name}: expected {'a pass' if passes else 'a failure'}, got {counts}")
        build(os.path.join(d, "nodebug-ref"), base, ["-g:none"])
        build(os.path.join(d, "nodebug"), base.replace("return 1", "return 2"), ["-g:none"])
        _, counts = compare(os.path.join(d, "nodebug-ref"), os.path.join(d, "nodebug"), "t")
        if counts[3] == 0:
            failures.append(f"a constant changed without a source file attribute: expected a failure, got {counts}")
        original = open(os.path.join(d, "ref", "C.class"), "rb").read()
        for name, broken in (("a truncated class file", original[:-3]), ("a zero byte after the bootstrap methods", padded_attribute(original, "BootstrapMethods"))):
            where = os.path.join(d, name.replace(" ", "-"))
            os.makedirs(where)
            open(os.path.join(where, "C.class"), "wb").write(broken)
            _, counts = compare(os.path.join(d, "ref"), where, "t")
            if counts[4] == 0:
                failures.append(f"{name}: expected it unread, got {counts}")
    for f in failures:
        print("FAIL " + f)
    print(f"classdiff self-test: {count - len(failures)} of {count} cases as expected")
    return 1 if failures else 0


def main():
    if sys.argv[1:] == ["--self-test"]:
        return self_test()
    if len(sys.argv) not in (3, 5):
        print("usage: tests/support/classdiff.py <reference dir> <dir> [--mode NAME] | --self-test", file=sys.stderr)
        return 2
    ref, new = sys.argv[1], sys.argv[2]
    mode = sys.argv[4] if len(sys.argv) == 5 and sys.argv[3] == "--mode" else os.path.basename(new.rstrip("/"))
    lines, (files, removed, added, other, unread) = compare(ref, new, mode)
    for line in lines:
        print(line)
    print(f"classdiff {mode}: {files} class files differ, {removed} members removed, {added} added, {other} other differences, {unread} not read whole")
    return 1 if other or unread else 0


if __name__ == "__main__":
    sys.exit(main())
