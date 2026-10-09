import re
def promote(path, fn_names):
    s = open(path).read()
    n = 0
    for name in fn_names:
        pat = re.compile(r"(\n(\s*))#\[inline\]\n(\s*)(pub(\(super\))? )?fn " + re.escape(name) + r"\(")
        s, k = pat.subn(lambda m: m.group(1) + "#[inline(always)]\n" + m.group(3) + (m.group(4) or "") + "fn " + name + "(", s)
        assert k == 1, (path, name, k)
        n += k
    open(path, "w").write(s)
    return n
total = promote("src/tir.rs", ["add", "add_pat", "add_str", "add_test", "add_fun"])
total += promote("src/typer/resolve.rs", ["complete_class"])
total += promote("src/typer/check.rs", ["completed_sig"])
total += promote("src/typer/overload.rs", ["settled"])
total += promote("src/arena.rs", ["eq"])
print("promoted", total)
