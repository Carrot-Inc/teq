#!/usr/bin/env python3
"""The profile of bench/pgo.sh's training made usable by a ship build for another target
(bench/cross-ship.sh): aarch64-apple-darwin unless named, x86_64-apple-darwin, aarch64-unknown-linux-gnu.

  translate <in.profdata> <out.profdata> [<target>]
      every function name of the profile with the crate disambiguators of the std crates it names
      (core, alloc, std, hashbrown, ...) replaced by the target's, as the installed toolchain's
      rlibs of the two targets spell them
  keys <profile.profdata> <instrumented aarch64-apple-darwin teq>
      the profile's records against the functions the Darwin build instruments, read from the
      binary's __llvm_prf_names and __llvm_prf_data: valid (the name and the control-flow hash
      match), mismatched (the name matches, the hash does not), passed over (another record of the
      function matches, in a profile merged from several builds), absent (no such function), and
      the functions without a record; each class also weighed by its execution counts

LLVM keys a function's profile by its name, for a local function prefixed with its module's name,
and checks a hash of its control flow. A Rust name (v0 mangling) carries the disambiguator of every
crate it names, and the std crates of each target are built with disambiguators of their own: in the
Linux profile `Vec<teq::X>::push` is `...Cs6i54tJFfzR_5alloc...`, in the Darwin build
`...CshxvaOLs88l5_5alloc...`, a name the profile does not have. teq's own crate and module names
follow from its `-C metadata`, which bench/cross-ship.sh pins to the native build's. The rewrite
parses each name (v0 grammar) and emits it again: a backref is a byte offset into the name, and the
disambiguators differ in length (alloc's 10 characters on Linux, 11 on Darwin), so every backref
after a replaced one moves.
"""
import collections, glob, hashlib, os, re, struct, subprocess, sys, tempfile, zlib

DARWIN = "aarch64-apple-darwin"
SYSROOT = subprocess.check_output(["rustc", "--print", "sysroot"], text=True).strip()
HOST = re.search(r"^host: (\S+)", subprocess.check_output(["rustc", "-vV"], text=True), re.M).group(1)
TOOLS = f"{SYSROOT}/lib/rustlib/{HOST}/bin"

# The v0 grammar (https://doc.rust-lang.org/rustc/symbol-mangling/v0.html), rewritten as it is read.
B62 = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
BASIC = set("abcdefhijlmnopstuvxyz")


class Unsupported(Exception):
    pass


def enc62(v):
    if v == 0:
        return "_"
    v, s = v - 1, ""
    while True:
        s, v = B62[v % 62] + s, v // 62
        if v == 0:
            return s + "_"


class Rewriter:
    """One symbol (without its `_R`) read and written again: crate roots found in `crates`, a map of
    (name, disambiguator) to a disambiguator, take the new one; each backref points where the node it
    named starts in the output. `roots` lists the crate roots read."""

    def __init__(self, s, crates):
        self.s, self.i, self.out, self.olen, self.starts, self.crates, self.roots = s, 0, [], 0, {}, crates, []

    def peek(self):
        return self.s[self.i] if self.i < len(self.s) else ""

    def emit(self, t):
        self.out.append(t)
        self.olen += len(t)

    def tag(self, t):
        self.i += 1
        self.emit(t)

    def b62(self):
        if self.peek() == "_":
            self.i += 1
            return 0
        v = 0
        while self.peek() != "_":
            if not self.peek() or self.peek() not in B62:
                raise Unsupported(f"base-62 number at {self.i}")
            v = v * 62 + B62.index(self.peek())
            self.i += 1
        self.i += 1
        return v + 1

    def decimal(self):
        start = self.i
        if self.peek() == "0":
            self.i += 1
            return 0
        while self.peek().isdigit():
            self.i += 1
        if start == self.i:
            raise Unsupported(f"decimal number at {self.i}")
        return int(self.s[start:self.i])

    def raw_ident(self):
        start = self.i
        if self.peek() == "u":
            self.i += 1
        n = self.decimal()
        if self.peek() == "_":
            self.i += 1
        body = self.s[self.i:self.i + n]
        self.i += n
        return self.s[start:self.i], body

    def disambiguator(self):
        start = self.i
        if self.peek() == "s":
            self.i += 1
            self.b62()
        return self.s[start:self.i]

    def identifier(self):
        d = self.disambiguator()
        self.emit(d + self.raw_ident()[0])

    def numbered(self, t):
        if self.peek() == t:
            start = self.i
            self.i += 1
            self.b62()
            self.emit(self.s[start:self.i])

    def backref(self):
        self.i += 1
        target = self.b62()
        if target not in self.starts:
            raise Unsupported(f"backref to {target}")
        self.emit("B" + enc62(self.starts[target]))

    def mark(self):
        self.starts.setdefault(self.i, self.olen)

    def path(self):
        self.mark()
        t = self.peek()
        if t == "B":
            return self.backref()
        if t == "C":
            self.i += 1
            d = self.disambiguator()
            raw, body = self.raw_ident()
            self.roots.append((body, d))
            self.emit("C" + self.crates.get((body, d), d) + raw)
        elif t == "M":
            self.tag("M"); self.impl_path(); self.type()
        elif t == "X":
            self.tag("X"); self.impl_path(); self.type(); self.path()
        elif t == "Y":
            self.tag("Y"); self.type(); self.path()
        elif t == "N":
            self.i += 1
            self.tag("N" + self.peek()); self.path(); self.identifier()
        elif t == "I":
            self.tag("I"); self.path()
            while self.peek() != "E":
                self.generic_arg()
            self.tag("E")
        else:
            raise Unsupported(f"path {t!r} at {self.i}")

    def impl_path(self):
        self.emit(self.disambiguator())
        self.path()

    def generic_arg(self):
        if self.peek() == "L":
            self.numbered("L")
        elif self.peek() == "K":
            self.tag("K"); self.const()
        else:
            self.type()

    def type(self):
        self.mark()
        t = self.peek()
        if t and t in BASIC:
            self.tag(t)
        elif t and t in "CMXYNI":
            self.path()
        elif t == "B":
            self.backref()
        elif t == "A":
            self.tag("A"); self.type(); self.const()
        elif t == "S" or t == "P" or t == "O":
            self.tag(t); self.type()
        elif t == "R" or t == "Q":
            self.tag(t); self.numbered("L"); self.type()
        elif t == "F":
            self.tag("F"); self.numbered("G")
            if self.peek() == "U":
                self.tag("U")
            if self.peek() == "K":
                self.tag("K")
                if self.peek() == "C":
                    self.tag("C")
                else:
                    self.emit(self.raw_ident()[0])
            while self.peek() != "E":
                self.type()
            self.tag("E"); self.type()
        elif t == "D":
            self.tag("D"); self.numbered("G")
            while self.peek() != "E":
                self.path()
                while self.peek() == "p":
                    self.tag("p"); self.emit(self.raw_ident()[0]); self.type()
            self.tag("E"); self.numbered("L")
        elif t == "T":
            self.tag("T")
            while self.peek() != "E":
                self.type()
            self.tag("E")
        else:
            raise Unsupported(f"type {t!r} at {self.i}")

    def const(self):
        self.mark()
        t = self.peek()
        if t == "B":
            self.backref()
        elif t == "p":
            self.tag("p")
        elif t == "R" or t == "Q":
            self.tag(t); self.const()
        elif t == "A" or t == "T":
            self.tag(t)
            while self.peek() != "E":
                self.const()
            self.tag("E")
        elif t == "V":
            self.tag("V"); self.path()
            f = self.peek()
            self.tag(f)
            if f == "T":
                while self.peek() != "E":
                    self.const()
                self.tag("E")
            elif f == "S":
                while self.peek() != "E":
                    self.identifier(); self.const()
                self.tag("E")
            elif f != "U":
                raise Unsupported(f"const fields {f!r} at {self.i}")
        elif t and t in BASIC:
            self.tag(t)
            start = self.i
            if self.peek() == "n":
                self.i += 1
            while self.peek() and self.peek() in "0123456789abcdef":
                self.i += 1
            if self.peek() != "_":
                raise Unsupported(f"const data at {self.i}")
            self.i += 1
            self.emit(self.s[start:self.i])
        else:
            raise Unsupported(f"const {t!r} at {self.i}")

    def symbol(self):
        if self.peek().isdigit():
            raise Unsupported("an encoding version")
        self.path()
        if self.peek() and self.peek() in "CMXYNIB":
            self.path()  # the instantiating crate
        self.emit(self.s[self.i:])  # a vendor suffix, `.llvm.<n>`
        return "".join(self.out)


def rewrite(name, crates):
    """A PGO name, `<module>;<symbol>` for a local function, with its symbol's crate roots replaced;
    (new name, crate roots read)."""
    prefix, sep, symbol = name.rpartition(";")
    if not symbol.startswith("_R"):
        return name, []
    r = Rewriter(symbol[2:], crates)
    return prefix + sep + "_R" + r.symbol(), r.roots


def crate_ids(target):
    """{crate name: disambiguator} of the target's std rlibs, the one their own symbols carry most"""
    out = {}
    for rlib in sorted(glob.glob(f"{SYSROOT}/lib/rustlib/{target}/lib/*.rlib")):
        name = re.match(r"lib(.+)-[0-9a-f]+\.rlib$", os.path.basename(rlib)).group(1)
        syms = subprocess.run([f"{TOOLS}/llvm-nm", "--defined-only", "-j", rlib], capture_output=True, text=True, timeout=120, check=True).stdout
        found = collections.Counter(re.findall(r"C(s[0-9A-Za-z]+_)" + str(len(name)) + re.escape(name), syms))
        if found:
            out[name] = found.most_common(1)[0][0]
    return out


def profdata(*args):
    subprocess.run([f"{TOOLS}/llvm-profdata", *args], check=True, timeout=300)


def text_names(lines):
    """the indices of the lines that hold a function name: a record's head, or the target of an
    indirect call or a vtable (`<name>:<count>`)"""
    for i, line in enumerate(lines):
        if i + 1 < len(lines) and lines[i + 1] == "# Func Hash:":
            yield i, line, ""
        else:
            m = re.match(r"(.*_R.*):(\d+)$", line)
            if m:
                yield i, m.group(1), ":" + m.group(2)


def translate(src, dst, target=DARWIN):
    darwin = crate_ids(target)
    with tempfile.TemporaryDirectory() as d:
        profdata("merge", "--text", "-o", f"{d}/in.proftext", src)
        lines = open(f"{d}/in.proftext").read().split("\n")
        # The profile's std is that of every target whose core it names: the host's (the Linux
        # training), the aarch64 trainer's, the target's own (a profile made on it, which needs no
        # translation), several of them in a profile merged from several builds. Each core met
        # is an installed target's, or the profile is refused.
        cores = collections.Counter(d for _, n, _ in text_names(lines) for c, d in rewrite(n, {})[1] if c == "core")
        if not cores:
            sys.exit("cross-profile: the profile names no function of core")
        targets = [HOST, target] + sorted(t for t in os.listdir(f"{SYSROOT}/lib/rustlib") if t not in (HOST, target))
        core_of = {t: crate_ids_core(t) for t in targets}
        sources = []
        for core in cores:
            source = next((t for t in targets if core_of[t] == core), None)
            if source is None:
                sys.exit(f"cross-profile: the profile names core {core} ({cores[core]} names), no installed target's; rustup target add the target it was trained on")
            sources.append(source)
        crates = {}
        for source in sources:
            ids = crate_ids(source)
            crates.update({(n, ids[n]): darwin[n] for n in ids if n in darwin})
        stats, unmapped = collections.Counter(), collections.Counter()
        known = set(crates) | {(n, i) for n, i in darwin.items()}
        for i, name, tail in text_names(lines):
            try:
                new, roots = rewrite(name, crates)
            except Unsupported as e:
                stats["unreadable"] += 1
                print(f"cross-profile: kept as it is, {e}: {name}", file=sys.stderr)
                continue
            stats["names"] += 1
            stats["changed" if new != name else "unchanged"] += 1
            unmapped.update((n, d) for n, d in roots if (n, d) not in known)
            lines[i] = new + tail
        # A std crate under an id no source names would keep its records from every function.
        stray = {(n, i): c for (n, i), c in unmapped.items() if n in darwin}
        if stray:
            sys.exit("cross-profile: std crates under ids of no installed target: " + ", ".join(f"{n} {i} ({c} names)" for (n, i), c in stray.items()))
        open(f"{d}/out.proftext", "w").write("\n".join(lines))
        profdata("merge", "-o", dst, f"{d}/out.proftext")
    print(f"cross-profile: {', '.join(f'{s} ({cores[core_of[s]]})' for s in sources)} to {target}: {stats['names']} names, {stats['changed']} rewritten, "
          f"{stats['unchanged']} as they were, {stats['unreadable']} unreadable")
    top = ", ".join(f"{n} {d} ({c})" for (n, d), c in unmapped.most_common(6))
    print(f"cross-profile: crate roots outside the std map (teq's own and the allocator shim's are expected): {top}")


def crate_ids_core(target):
    syms = glob.glob(f"{SYSROOT}/lib/rustlib/{target}/lib/libcore-*.rlib")
    if not syms:
        return None
    out = subprocess.run([f"{TOOLS}/llvm-nm", "--defined-only", "-j", syms[0]], capture_output=True, text=True, timeout=120).stdout
    found = collections.Counter(re.findall(r"C(s[0-9A-Za-z]+_)4core", out))
    return found.most_common(1)[0][0] if found else None


def section(binary, name):
    macho = open(binary, "rb").read(4) == b"\xcf\xfa\xed\xfe"
    with tempfile.TemporaryDirectory() as d:
        sec = f"__DATA,{name}" if macho else name
        subprocess.run([f"{TOOLS}/llvm-objcopy", "--dump-section", f"{sec}={d}/s", binary, f"{d}/o"], check=True, timeout=300)
        return open(f"{d}/s", "rb").read()


def uleb(b, i):
    v = s = 0
    while True:
        c = b[i]
        i += 1
        v |= (c & 0x7F) << s
        s += 7
        if c < 0x80:
            return v, i


def instrumented(binary):
    """{name: control-flow hash} of the functions an instrumented binary counts"""
    return prf_sections(section(binary, "__llvm_prf_names"), section(binary, "__llvm_prf_data"), binary)


def prf_sections(blob, data, what):
    """The names of __llvm_prf_names (zlib blocks of names joined by \\x01) and the hashes of
    __llvm_prf_data. Every version of LLVM's raw profile so far opens a function's record with its
    name's MD5 and its control-flow hash, but the record's size moves with the version (64 bytes
    under version 10, LLVM 19 to 22; 72 under LLVM 23): the size taken is the smallest whose every
    record opens with the MD5 of a name of the binary (a multiple of it passes as well), and a
    section no size reads so is refused."""
    names, i = [], 0
    while i < len(blob):
        n, i = uleb(blob, i)
        c, i = uleb(blob, i)
        names += (zlib.decompress(blob[i:i + c]) if c else blob[i:i + n]).split(b"\x01")
        i += c or n
        while i < len(blob) and blob[i] == 0:
            i += 1
    by_md5 = {struct.unpack("<Q", hashlib.md5(n).digest()[:8])[0]: n.decode() for n in names if n}
    size = next((s for s in range(16, 257, 8) if data and len(data) % s == 0
                 and all(struct.unpack_from("<Q", data, k)[0] in by_md5 for k in range(0, len(data), s))), None)
    if size is None:
        sys.exit(f"cross-profile: the __llvm_prf_data of {what} ({len(data)} bytes) reads as records of no size up to 256 bytes")
    out = {}
    for k in range(0, len(data), size):
        md5, h = struct.unpack_from("<QQ", data, k)
        out[by_md5[md5]] = h
    return out


def records(path):
    """[(name, hash, sum of counters)] of a profile, a record each: a profile merged from several
    builds holds a function under each control-flow hash it was counted with"""
    with tempfile.TemporaryDirectory() as d:
        profdata("merge", "--text", "-o", f"{d}/p.proftext", path)
        lines = open(f"{d}/p.proftext").read().split("\n")
    out = []
    for i, line in enumerate(lines):
        if i + 1 < len(lines) and lines[i + 1] == "# Func Hash:":
            h, n = int(lines[i + 2]), int(lines[i + 4])
            out.append((line, h, sum(int(x) for x in lines[i + 6:i + 6 + n])))
    return out


def keys(profile, binary):
    """A record is valid when its function has its hash; one whose function has another hash is
    mismatched, unless another record of the function has it (LLVM takes that one): passed over."""
    prof, inst = records(profile), instrumented(binary)
    taken = {name for name, h, _ in prof if inst.get(name) == h}
    count, weight = collections.Counter(), collections.Counter()
    for name, h, w in prof:
        c = "absent" if name not in inst else "valid" if inst[name] == h else "passed over" if name in taken else "mismatched"
        count[c] += 1
        weight[c] += w
    names = {name for name, _, _ in prof}
    without = sum(1 for n in inst if n not in names)
    total = sum(weight.values()) or 1
    print(f"cross-profile: {len(prof)} records of {len(names)} functions, {len(inst)} functions instrumented in {binary}")
    for c in ("valid", "mismatched", "passed over", "absent"):
        if count[c] or c != "passed over":
            print(f"  {c:11} {count[c]:6} records, {weight[c] / total * 100:6.2f}% of the counts")
    print(f"  {'missing':11} {without:6} functions instrumented without a record")


if __name__ == "__main__":
    if len(sys.argv) in (4, 5) and sys.argv[1] == "translate":
        translate(sys.argv[2], sys.argv[3], *sys.argv[4:])
    elif len(sys.argv) == 4 and sys.argv[1] == "keys":
        keys(sys.argv[2], sys.argv[3])
    else:
        sys.exit(__doc__)
