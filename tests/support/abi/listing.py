#!/usr/bin/env python3
"""tests/support/abi/listing.py <dir> [names...]: the layout of the class files under <dir> as javap shows it, a line
per class, interface, field and method: `<class> class`, `<class> implements <interface>`, `<class>
<name>:<descriptor> <flags>` for a field and `<class> <name><descriptor> <flags>` for a method, sorted. With names,
only the members of those names and the interfaces of those binary names are listed, the members of that name in
the classes of that simple binary name for `Owner#name` (with a descriptor's beginning, `Owner#name(desc`, the
members of that name and descriptor; with `+final` after `Owner#name`, an instance field's `final`, which the
listing leaves out otherwise), and the class lines when `classes` is among them. The corpus of tests/support/abi compares
teq's listing of a program with scalac 3.8.4's, which <name>.expected holds: `listing.py <scalac's output> <names>`
over `scala-cli compile -S 3.8.4 --jvm system --server=false <name>.scala --compilation-output <dir>`."""
import os
import struct
import sys

FLAGS = [(0x0001, "public"), (0x0002, "private"), (0x0004, "protected"), (0x0008, "static"), (0x0010, "final"),
         (0x0040, "bridge"), (0x0080, "varargs"), (0x0100, "native"), (0x0200, "interface"), (0x0400, "abstract"),
         (0x1000, "synthetic"), (0x4000, "enum")]


def flags(access, member, final=False):
    # 0x0020 is ACC_SUPER on a class and ACC_SYNCHRONIZED on a method; on a field 0x0040 is ACC_VOLATILE, listed as
    # `volatile`, and 0x0080 ACC_TRANSIENT, left out. An instance field is listed without ACC_FINAL, which teq leaves
    # off most, but where `final` asks for it.
    instance_field = member == "field" and not access & 0x0008 and not final
    named = [(bit, "volatile" if bit == 0x0040 and member == "field" else name) for bit, name in FLAGS]
    out = [name for bit, name in named if access & bit and not (bit == 0x0080 and member == "field") and not (bit == 0x0010 and instance_field)]
    return ",".join(out) or "-"


def read(path):
    data = open(path, "rb").read()
    at = [8]

    def u(n):
        v = int.from_bytes(data[at[0]:at[0] + n], "big")
        at[0] += n
        return v
    count = u(2)
    pool = [None] * count
    i = 1
    while i < count:
        tag = u(1)
        if tag == 1:
            n = u(2)
            pool[i] = data[at[0]:at[0] + n].decode("utf-8", "surrogateescape")
            at[0] += n
        elif tag in (3, 4):
            u(4)
        elif tag in (5, 6):
            u(8)
            i += 1
        elif tag in (7, 8, 16, 19, 20):
            pool[i] = ("ref", u(2))
        elif tag in (9, 10, 11, 12, 17, 18):
            u(4)
        elif tag == 15:
            u(3)
        else:
            raise ValueError(f"{path}: constant pool tag {tag}")
        i += 1

    def cls(k):
        return pool[pool[k][1]]
    access = u(2)
    this = cls(u(2))
    u(2)
    interfaces = [cls(u(2)) for _ in range(u(2))]
    lines = [(this, "class", f"{this} class", f"{this} class")]
    lines += [(this, i, f"{this} implements {i}", f"{this} implements {i}") for i in interfaces]
    for kind in ("field", "method"):
        for _ in range(u(2)):
            acc, name, desc = u(2), pool[u(2)], pool[u(2)]
            for _ in range(u(2)):
                u(2)
                n = u(4)
                at[0] += n
            sep = ":" if kind == "field" else ""
            lines.append((this, name, f"{this} {name}{sep}{desc} {flags(acc, kind)}", f"{this} {name}{sep}{desc} {flags(acc, kind, True)}"))
    return lines


def main():
    root, names = sys.argv[1], set(sys.argv[2:])
    # `Owner#name+final`: `Owner#name` with an instance field's ACC_FINAL listed.
    finals = {tuple(n[:-len("+final")].split("#", 1)) for n in names if "#" in n and n.endswith("+final")}
    names = {n[:-len("+final")] if n.endswith("+final") else n for n in names}
    # `Owner#name`: the members of that name in the classes of that simple binary name only;
    # `Owner#name(desc`: the members whose name and descriptor begin so.
    owned = {tuple(n.split("#", 1)) for n in names if "#" in n and "(" not in n}
    prefixed = [tuple(n.split("#", 1)) for n in names if "#" in n and "(" in n]
    names = {n for n in names if "#" not in n}
    out = []
    for d, _, files in os.walk(root):
        for f in files:
            # The classes of teq's runtime a module's products hold are no part of the program's layout.
            if f.endswith(".class") and not os.path.relpath(os.path.join(d, f), root).startswith("scala" + os.sep):
                for this, name, line, final_line in read(os.path.join(d, f)):
                    simple = this.rsplit("/", 1)[-1]
                    member = line.split(" ", 2)[1]
                    if (not names and not owned and not prefixed) or name in names or (simple, name) in owned or (name == "class" and "classes" in names) \
                            or any(simple == o and member.startswith(m) for o, m in prefixed):
                        out.append(final_line if (simple, name) in finals else line)
    for line in sorted(out):
        print(line)


if __name__ == "__main__":
    main()
