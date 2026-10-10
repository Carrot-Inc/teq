# The comparisons of check-export.sh, and teq.lock's third writer and a reader of it.
#   fixture <export> <fixture>: the export, its header (teq and binaries) the fixture's, is the
#     fixture's bytes.
#   header <export> <export>: the two pin the same compiler and the same binaries, an empty table matching an
#     empty one alone (a release not published yet: check-export.sh asks the export's log that the release is
#     absent, so that an export that could not read it matches nothing).
#   binaries <export> <version> <record>...: the export pins the release, `teq` the version and its binaries
#     exactly the records given, as pin's (bench/release-smoke.sh).
#   pin <export> <fixture> <version> <record>...: the export after bench/release.sh --pin pins the release:
#     `teq` the version and its binaries exactly the records given, `<classifier> <url> <sha1> <size>` each (the
#     release's full URL, the repository's .sha1, the size it serves), at least one, no other classifier; and
#     is the fixture's bytes but for its header and, in its inputs, the digests of project/plugins.sbt and
#     build.sbt (the plugin and the compiler the pin moves) and the hash over them.
#   dependency <before> <after> <project> <org:name:version>: the after export is the before one with
#     the jar's key added to the project's compile, runtime and test classpaths and, when the before
#     one's table has no record of the key, that record added to the table; its other lines unchanged.
#   alias <export> <url> <name> <key> <project>...: the repository at the URL, which the projects'
#     resolvers name differently, is listed once, by the name the first project gives it, and the
#     table's one record of the key names it, which each project's three classpaths name once.
#   mirror <export> <url> <name> <key>...: the build's one repository, the mirror at the URL by its
#     name, is the export's one, and the keys' records (sbt's own scala-library among them) name it.
#   classpaths <export> <sbt log>: every configuration's classpath but its products, each key's file
#     at the path its record names or the Maven layout of the key under its repository's place in
#     coursier's cache, is the externalDependencyClasspath sbt's `export` printed for it, in order, the
#     files of the pinned sha1 and size (sbt's own scala-library, outside the cache, by its bytes).
#   projects <export>: the projects' names; configurations <export>: the configurations, as sbt's
#     scopes (`api/Compile`), in the order of the commands check-export.sh gives sbt.
#   recorded <export> <before>: what check-export.sh's recorded scenario has sbt alone run is in the
#     export: api's generators of another kind (its buildInfo of an action other than gitSha among
#     them, with the reason) and sbt's managed directories among its sources, its test generator,
#     its resource generator and mirrored's, its test options and the stage it does not reproduce,
#     without a stage block; every other project's block the before export's.
import difflib
import hashlib
import os
import sys


# teq.lock (docs/TARGETS.md, "The export and the project verbs"): its canonical form, which sbt-teq's
# Lock.scala and teq's task::lock::write write too, and a reader of the subset, as task::lock reads it.
HEADER = ("teq", "format", "binaries")
# The places whose records are written on one line, unless a record holds a list: a classpath's
# entries and a stage layer's mappings (None matches any key).
RECORDS = (("projects", None, "configurations", None, "classpath", None), ("projects", None, "stage", "layers", None, None))
RESERVED = set("null Null NULL true True TRUE false False FALSE yes Yes YES no No NO on On ON off Off OFF y Y n N .inf .Inf .INF .nan .NaN .NAN".split())
MAX_KEY = 1024
MAX_DEPTH = 64
MAX_INT = 2**53 - 1
DIGITS = "0123456789"
LETTERS = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"


def word(s):
    return bool(s) and all(c in LETTERS or c in DIGITS or c in "_./+-:@" for c in s) and (s[0] in LETTERS or s[0] in DIGITS or s[0] in "_./") and s[-1] != ":"


def number_like(s):
    led = s[0] in DIGITS or (s[0] == "." and len(s) > 1 and s[1] in DIGITS)
    return led and all(c in DIGITS or c in "._:-+eEtTzZ" for c in s) and s.count(".") <= 1


def radix(s):
    return len(s) > 2 and s[0] == "0" and s[1] in "xob" and all(c in DIGITS or c in "abcdefABCDEF_" for c in s[2:])


def plain(s):
    """A string written plain as a key or a flow mapping's value."""
    return word(s) and s not in RESERVED and not number_like(s) and not radix(s)


def plain_line(s):
    """A string written plain on a line of its own: words joined by single spaces, the first plain."""
    words = s.split(" ")
    return plain(words[0]) and all(word(w) for w in words)


def unprinted(c):
    o = ord(c)
    return o < 0x20 or 0x7F <= o <= 0x9F or o in (0x2028, 0x2029, 0xFEFF, 0xFFFE, 0xFFFF)


def quote(s):
    escapes = {'"': '\\"', "\\": "\\\\", "\n": "\\n", "\r": "\\r", "\t": "\\t"}
    return '"' + "".join(escapes.get(c) or (f"\\u{ord(c):04x}" if unprinted(c) else c) for c in s) + '"'


def key_text(k):
    return k if plain(k) else quote(k)


def holds_list(v):
    return isinstance(v, list) or isinstance(v, dict) and any(holds_list(x) for x in v.values())


def is_record(path, value):
    shaped = any(len(p) == len(path) and all(a is None or a == b for a, b in zip(p, path)) for p in RECORDS)
    return shaped and isinstance(value, dict) and not holds_list(value)


def ordered(fields, root):
    rank = (lambda k: HEADER.index(k) if k in HEADER else len(HEADER)) if root else (lambda k: 0)
    return sorted(fields.items(), key=lambda kv: (rank(kv[0]), kv[0].encode()))


def scalar(value, line):
    if isinstance(value, bool):
        return "true" if value else "false"
    if isinstance(value, int):
        return str(value)
    if isinstance(value, str):
        return value if (plain_line(value) if line else plain(value)) else quote(value)
    if value == []:
        return "[]"
    if value == {}:
        return "{}"
    raise ValueError(f"teq.lock holds no {value!r}")


def flow(value):
    if isinstance(value, dict) and value:
        return "{" + ", ".join(key_text(k) + ": " + flow(v) for k, v in ordered(value, False)) + "}"
    return scalar(value, False)


def on_line(value, path):
    if is_record(path, value):
        return flow(value)
    if isinstance(value, (dict, list)) and value:
        return None
    return scalar(value, True)


def block(value, depth, path):
    indent = "  " * depth
    lines = []
    if isinstance(value, dict):
        for k, v in ordered(value, not path):
            text = on_line(v, path + (k,))
            if text is not None:
                lines.append(f"{indent}{key_text(k)}: {text}")
            else:
                lines.append(f"{indent}{key_text(k)}:")
                lines += block(v, depth + 1, path + (k,))
    else:
        for v in value:
            text = on_line(v, path + ("*",))
            if text is not None:
                lines.append(f"{indent}- {text}")
            else:
                inner = block(v, depth + 1, path + ("*",))
                lines.append(f"{indent}- {inner[0][len(indent) + 2:]}")
                lines += inner[1:]
    return lines


def canonical(value):
    """The lock's canonical text of a tree, its trailing newline included."""
    return "\n".join(block(value, 0, ())) + "\n"


class LockError(Exception):
    pass


def parse(text):
    """The tree of a lock's text, the subset alone: a failure names the line."""
    if not text.endswith("\n"):
        raise LockError(f"line {max(text.count(chr(10)) + 1, 1)}: {'an empty file' if not text else 'no newline at the end'}")
    lines = [l[:-1] if l.endswith("\r") else l for l in text[:-1].split("\n")]
    state = {"pos": 0, "inner": None}

    def fail(line, message):
        raise LockError(f"line {line}: {message}")

    def current():
        if state["pos"] >= len(lines):
            return None
        line = lines[state["pos"]]
        if state["inner"] is not None:
            return state["inner"], line[state["inner"]:]
        content = line.lstrip(" ")
        if not content:
            fail(state["pos"] + 1, "a blank line")
        if content[0] == "\t":
            fail(state["pos"] + 1, "a tab in the indentation")
        return len(line) - len(content), content

    def advance():
        state["pos"] += 1
        state["inner"] = None

    def element(content):
        return content == "-" or content.startswith("- ")

    def quoted(s, line):
        out, i = [], 1
        escapes = {'"': '"', "\\": "\\", "n": "\n", "r": "\r", "t": "\t"}
        while i < len(s):
            c = s[i]
            if c == '"':
                return "".join(out), i + 1
            if c == "\\":
                e = s[i + 1:i + 2]
                if e in escapes:
                    out.append(escapes[e])
                    i += 2
                elif e == "u":
                    h = s[i + 2:i + 6]
                    if len(h) != 4 or not all(x in "0123456789abcdefABCDEF" for x in h) or 0xD800 <= int(h, 16) <= 0xDFFF:
                        fail(line, f"the escape \\u{h}, which is no character")
                    out.append(chr(int(h, 16)))
                    i += 6
                elif e:
                    fail(line, f"the escape \\{e}, which the lock does not use")
                else:
                    break
            elif unprinted(c):
                fail(line, f"the character U+{ord(c):04X} in quotes, which the lock escapes")
            else:
                out.append(c)
                i += 1
        fail(line, "a quoted string not closed on its line")

    def bare(token, line):
        if token in ("true", "false"):
            return token == "true"
        digits = token[1:] if token.startswith("-") else token
        if digits and all(c in DIGITS for c in digits) and (digits == "0" or digits[0] != "0") and token != "-0":
            if abs(int(token)) > MAX_INT:
                fail(line, f"the integer {token} is out of range")
            return int(token)
        if plain(token):
            return token
        fail(line, f"the bare `{token}`, which YAML reads as other than this text: quote it")

    def flow_map(s, at, line, depth):
        if depth > MAX_DEPTH:
            fail(line, f"nested deeper than {MAX_DEPTH} levels")
        pos, entries = at + 1, {}
        if s[pos:].startswith("}"):
            return entries, pos + 1
        while True:
            if s[pos:].startswith('"'):
                k, n = quoted(s[pos:], line)
                end = pos + n
            else:
                n = s.find(": ", pos)
                end = n if n >= 0 else len(s)
                k = s[pos:end]
                if not plain(k):
                    fail(line, f"the bare key `{k}` in a flow mapping, which YAML may read otherwise: quote it")
            if not s[end:].startswith(": "):
                fail(line, f"no `: ` after the key {key_text(k)} in a flow mapping")
            if len(s[pos:end]) > MAX_KEY:
                fail(line, f"a key of more than YAML's {MAX_KEY} characters")
            if k in entries:
                fail(line, f"the key {key_text(k)} a second time")
            pos = end + 2
            if s[pos:].startswith("{"):
                v, pos = flow_map(s, pos, line, depth + 1)
            elif s[pos:].startswith('"'):
                v, n = quoted(s[pos:], line)
                pos += n
            else:
                n = min([i for i in (s.find(",", pos), s.find("}", pos), s.find(" ", pos)) if i >= 0] or [len(s)])
                v = bare(s[pos:n], line)
                pos = n
            entries[k] = v
            if s[pos:].startswith(", "):
                pos += 2
            elif s[pos:].startswith("}"):
                return entries, pos + 1
            else:
                fail(line, "a flow mapping not closed by `}`")

    def inline(s, line, depth):
        if s.startswith("{"):
            v, end = flow_map(s, 0, line, depth + 1)
            if end != len(s):
                fail(line, f"`{s[end:]}` after a flow mapping")
            return v
        if s.startswith("["):
            if s == "[]":
                return []
            fail(line, "a flow list, which the lock writes as `[]` alone")
        if s.startswith('"'):
            v, end = quoted(s, line)
            if end != len(s):
                fail(line, f"`{s[end:]}` after a quoted string")
            return v
        if " " in s:
            if plain_line(s):
                return s
            fail(line, f"the bare text `{s}`, which YAML may read otherwise: quote it")
        return bare(s, line)

    def key(content, line):
        if content.startswith('"'):
            k, end = quoted(content, line)
        else:
            token = content.split(" ")[0]
            if not token.endswith(":"):
                fail(line, f"`{content}` where a key and its colon stand")
            k, end = token[:-1], len(token) - 1
            if not plain(k):
                fail(line, f"the bare key `{k}`, which YAML reads as other than this text: quote it")
        if len(content[:end]) > MAX_KEY:
            fail(line, f"a key of more than YAML's {MAX_KEY} characters")
        if not content[end:].startswith(":"):
            fail(line, f"no colon after the key {key_text(k)}")
        return k, content[end + 1:]

    def is_entry(s):
        if s.startswith('"'):
            try:
                _, end = quoted(s, 0)
            except LockError:
                return False
            return s[end:].startswith(":")
        return not s.startswith("{") and s.split(" ")[0].endswith(":")

    def parse_block(column, depth):
        if depth > MAX_DEPTH:
            fail(state["pos"] + 1, f"nested deeper than {MAX_DEPTH} levels")
        found = current()
        if found is None:
            fail(state["pos"] + 1, "the end where a block stands")
        if found[0] != column:
            fail(state["pos"] + 1, f"indented {found[0]} spaces where its block is at {column}")
        return parse_list(column, depth) if element(found[1]) else parse_mapping(column, depth)

    def parse_mapping(column, depth):
        entries = {}
        while (found := current()) is not None:
            indent, content = found
            line = state["pos"] + 1
            if indent < column:
                break
            if indent > column:
                fail(line, f"indented {indent} spaces where its mapping's entries are at {column}")
            if element(content):
                fail(line, "a list's element among a mapping's entries")
            k, rest = key(content, line)
            if k in entries:
                fail(line, f"the key {key_text(k)} a second time")
            advance()
            if not rest:
                below = current()
                if below is None or below[0] <= column:
                    fail(line, f"{key_text(k)} with nothing below it")
                entries[k] = parse_block(column + 2, depth + 1)
            elif rest.startswith(" "):
                entries[k] = inline(rest[1:], line, depth)
            else:
                fail(line, "no space after the key's colon")
        return entries

    def parse_list(column, depth):
        items = []
        while (found := current()) is not None:
            indent, content = found
            line = state["pos"] + 1
            if indent < column:
                break
            if indent > column:
                fail(line, f"indented {indent} spaces where its list's elements are at {column}")
            if not content.startswith("- "):
                fail(line, "an element with nothing after its dash" if content == "-" else "a mapping's entry among a list's elements")
            rest = content[2:]
            if element(rest) or is_entry(rest):
                state["inner"] = column + 2
                items.append(parse_block(column + 2, depth + 1))
            else:
                advance()
                items.append(inline(rest, line, depth))
        return items

    first = current()
    if first is None or first[0] != 0 or first[1].startswith("-"):
        fail(1, "the lock is no mapping at its first column")
    return parse_mapping(0, 0)


def read(path):
    with open(path, encoding="utf-8", newline="") as f:
        return f.read()


def load(path):
    return parse(read(path))


def show_diff(a, b, a_name, b_name):
    sys.stdout.writelines(list(difflib.unified_diff(a.splitlines(True), b.splitlines(True), a_name, b_name))[:80])


def table(lock):
    """The lock's binaries: an absent table is an empty one, as every reader takes it (a bare `binaries:` is
    no lock's, refused by the parser)."""
    return lock.get("binaries", {})


def fixture(export_path, fixture_path):
    export, wanted = load(export_path), read(fixture_path)
    before = parse(wanted)
    export.update({"teq": before["teq"], "binaries": table(before)})
    text = canonical(export)
    if text != wanted:
        show_diff(wanted, text, fixture_path, export_path)
        return False
    return True


def pins(export, export_path, version, records):
    """Whether the export pins the release: `teq` the version, its binaries exactly the records, `<classifier>
    <url> <sha1> <size>` each, at least one; prints what differs."""
    binaries = export["binaries"] if isinstance(export["binaries"], dict) else {}
    expected = {}
    for record in records:
        fields = record.split(" ")
        if len(fields) != 4 or not fields[3].isdigit() or len(fields[2]) != 40 or any(c not in "0123456789abcdef" for c in fields[2]):
            print(f"the record '{record}' is not <classifier> <url> <sha1> <size>")
            return False
        expected[fields[0]] = " ".join(fields[1:])
    if not expected:
        print("no record of the release's binaries to compare the export with")
        return False
    if export["teq"] != version or binaries != expected:
        print(f"{export_path} pins {export['teq']}, not {version}'s binaries as the repository serves them:")
        for c in sorted(set(binaries) | set(expected)):
            if binaries.get(c) != expected.get(c):
                print(f"  {c}: the export {binaries.get(c, 'none')}, the release {expected.get(c, 'none')}")
        return False
    return True


def binaries(export_path, version, *records):
    return pins(load(export_path), export_path, version, records)


def pin(export_path, fixture_path, version, *records):
    export, wanted = load(export_path), read(fixture_path)
    if not pins(export, export_path, version, records):
        return False
    before = parse(wanted)
    export.update({k: before[k] for k in ("teq", "binaries")})
    for moved in ("project/plugins.sbt", "build.sbt"):
        export["inputs"]["files"][moved] = before["inputs"]["files"][moved]
    export["inputs"]["sha256"] = before["inputs"]["sha256"]
    text = canonical(export)
    if text != wanted:
        show_diff(wanted, text, fixture_path, export_path)
        return False
    return True


def header(a_path, b_path):
    a, b = load(a_path), load(b_path)
    if (a["teq"], table(a)) != (b["teq"], table(b)):
        print(f"{a_path} pins {a['teq']} {table(a) or 'no binary'}, {b_path} {b['teq']} {table(b) or 'no binary'}")
        return False
    return True


def dependency(before_path, after_path, project, key):
    before, after = read(before_path), read(after_path)
    added, earlier = parse(after), parse(before)
    for configuration in ("compile", "runtime", "test"):
        classpath = added["projects"][project]["configurations"][configuration]["classpath"]
        matching = [e for e in classpath if e == key]
        if len(matching) != 1:
            print(f"{project}/{configuration} holds {key} {len(matching)} times")
            return False
        classpath.remove(key)
    recorded = key in earlier["jars"]
    if not recorded:
        if key not in added["jars"]:
            print(f"the table holds no record of {key}")
            return False
        del added["jars"][key]
    if canonical(added) != before:
        show_diff(before, canonical(added), before_path, after_path + " without the dependency")
        return False
    # Every line the dependency changed lies within the project's block, but the table's one new
    # record of the key.
    lines, later = before.splitlines(), after.splitlines()
    start = lines.index(f"  {key_text(project)}:", lines.index("projects:"))
    end = next((i for i in range(start + 1, len(lines)) if not lines[i].startswith("    ")), len(lines))
    record = f"  {key_text(key)}: "
    records = 0
    for tag, i1, i2, j1, j2 in difflib.SequenceMatcher(None, lines, later, autojunk=False).get_opcodes():
        if tag == "equal" or start <= i1 and i2 <= end:
            continue
        added_lines = [later[j] for j in range(j1, j2) if later[j].startswith(record)]
        kept = [later[j] for j in range(j1, j2) if not later[j].startswith(record)]
        if not recorded and added_lines and kept == lines[i1:i2]:
            records += len(added_lines)
        else:
            print(f"a change outside {project}'s block, at line {i1 + 1}")
            return False
    if records != (0 if recorded else 1):
        print(f"the table gained {records} records of {key}")
        return False
    return True


def fields(export, key):
    """A record of the jar table: its repository, sha1, size and path (the Maven layout's when it names none)."""
    repository, sha1, size, *path = export["jars"][key].split(" ")
    return repository, sha1, int(size), path[0] if path else maven_path(key)


def alias(export_path, url, name, key, *projects):
    export = load(export_path)
    failures = []
    listed = [r["id"] for r in export["repositories"] if r["url"] == url]
    if listed != [name]:
        failures.append(f"the repositories list {url} as {listed}, not once as {name}")
    record = export["jars"].get(key)
    if not record or fields(export, key)[0] != name:
        failures.append(f"the table's record of {key} is {record}")
    for project in projects:
        for configuration in ("compile", "runtime", "test"):
            times = export["projects"][project]["configurations"][configuration]["classpath"].count(key)
            if times != 1:
                failures.append(f"{project}/{configuration} names {key} {times} times")
    print("\n".join(failures))
    return not failures


def mirror(export_path, url, name, *keys):
    export = load(export_path)
    failures = []
    listed = [(r["id"], r["url"]) for r in export["repositories"]]
    if listed != [(name, url)]:
        failures.append(f"the repositories are {listed}, not {name} at {url} alone")
    for key in keys:
        record = export["jars"].get(key)
        if not record or fields(export, key)[0] != name:
            failures.append(f"the table's record of {key} is {record}")
    print("\n".join(failures))
    return not failures


def maven_path(key):
    parts = key.split(":")
    if len(parts) not in (3, 4) or not all(parts):
        return None
    organization, name, version = parts[:3]
    return f"{organization.replace('.', '/')}/{name}/{version}/{name}-{version}{''.join('-' + c for c in parts[3:])}.jar"


def coursier_file(cache, url):
    """The file coursier keeps for a URL, as its CachePath.localFile names it with no user: the scheme, a `/`, then
    the URL after the scheme's `:` with its leading `/`s taken off and a trailing `/` made `/.directory`, the whole
    escaped by coursier_escape (the authority's user, a port, the query and the fragment kept); None for a URL
    coursier refuses: no scheme, no `/` after it, or a `.` or `..` segment (tests/support/coursier-files.txt)."""
    scheme, colon, rest = url.partition(":")
    if not colon:
        return None
    if rest.startswith("///"):
        rest = rest[3:]
    elif rest.startswith("/"):
        rest = rest[1:]
    else:
        return None
    if rest.endswith("/"):
        rest += ".directory"
    place = coursier_escape(scheme + "/" + rest.lstrip("/"))
    if any(segment in (".", "..") for segment in place.split("/")):
        return None
    return os.path.join(cache, *place.split("/"))


def coursier_escape(text):
    """coursier's CachePath.escape: every UTF-16 unit above 128 and each of ` %$&+,:;=?@<>#` as `%`
    and two digits of its value in base 16 (a repository's port, a version's `+`)."""
    units = text.encode("utf-16-be")
    out = []
    for i in range(0, len(units), 2):
        u = int.from_bytes(units[i:i + 2], "big")
        digit = lambda n: chr(ord("0") + n if n < 10 else ord("A") + n - 10)
        out.append("%" + digit(u // 16) + digit(u % 16) if u > 128 or chr(u) in " %$&+,:;=?@<>#" else chr(u))
    return "".join(out)


def configurations(export):
    """The export's configurations in the order of the commands check-export.sh gives sbt."""
    return [(p, c) for p in export["projects"] for c in export["projects"][p]["configurations"]]


def classpaths(export_path, log_path):
    export = load(export_path)
    root = os.path.dirname(os.path.abspath(export_path))
    cache = os.environ.get("COURSIER_CACHE") or os.path.expanduser("~/Library/Caches/Coursier/v1" if sys.platform == "darwin" else "~/.cache/coursier/v1")
    places = {"${CSR_CACHE}": cache, "${BASE}": root, "${SBT_BOOT}": os.path.expanduser("~/.sbt/boot")}
    def local(entry):
        for placeholder, place in places.items():
            if entry.startswith(placeholder):
                return os.path.normpath(place + entry[len(placeholder):])
        return os.path.normpath(entry)
    printed = [line.strip() for line in read(log_path).splitlines() if line.startswith("List(")]
    lists = [[local(e) for e in line[len("List("):-1].split(", ") if e] for line in printed]
    named = configurations(export)
    if len(lists) != len(named):
        print(f"sbt printed {len(lists)} classpaths for the export's {len(named)} configurations")
        return False
    urls = {r["id"]: r["url"] for r in export["repositories"]}
    failures = []
    jars = 0
    for (project, configuration), sbts in zip(named, lists):
        ours = [e for e in export["projects"][project]["configurations"][configuration]["classpath"] if isinstance(e, str) or "file" in e]
        where = f"{project}/{configuration}"
        if len(ours) != len(sbts):
            failures.append(f"{where}: the export has {len(ours)} jars and files, sbt {len(sbts)}")
            continue
        for i, (entry, file) in enumerate(zip(ours, sbts)):
            if not isinstance(entry, str):
                if os.path.normpath(os.path.join(root, entry["file"])) != file:
                    failures.append(f"{where}[{i}]: the export's file {entry['file']}, sbt's {file}")
                continue
            jars += 1
            repository, sha1, size, path = fields(export, entry)
            derived = os.path.normpath(coursier_file(cache, urls[repository] + path))
            if file.startswith(os.path.normpath(cache) + os.sep) and file != derived:
                failures.append(f"{where}[{i}]: {entry} at {derived}, sbt's {file}")
            elif not file.startswith(os.path.normpath(cache) + os.sep) and not entry.startswith("org.scala-lang:"):
                failures.append(f"{where}[{i}]: {entry} is sbt's {file}, outside coursier's cache")
            data = open(file, "rb").read()
            if (hashlib.sha1(data).hexdigest(), len(data)) != (sha1, size):
                failures.append(f"{where}[{i}]: {file} is not the bytes the export pins for {entry}")
    print("\n".join(failures[:40]) if failures else f"{len(named)} configurations, {jars} jars")
    return not failures


def projects(export_path):
    print(" ".join(load(export_path)["projects"]))
    return True


def scopes(export_path):
    print(" ".join(f"{p}/{k.capitalize()}" for p, k in configurations(load(export_path))))
    return True


def recorded(export_path, before_path):
    export, before = load(export_path), load(before_path)
    projects = export["projects"]
    api, mirrored = projects["api"], projects["mirrored"]
    compile, test = api["configurations"]["compile"], api["configurations"]["test"]
    unnamed = {"kind": "sbt", "task": "an unnamed task"}
    build_info = [g for g in compile.get("generators", []) if g.get("task") == "buildInfo"]
    unsupported = api.get("unsupported", {})
    wanted = [
        ("api's compile generators hold an unnamed task's", unnamed in compile.get("generators", [])),
        ("api's buildInfo is sbt's, with its reason", len(build_info) == 1 and build_info[0].get("kind") == "sbt" and "builtAt" in build_info[0].get("reason", "")),
        ("api's compile sources are sbt's managed directory and not teq's", any(s.endswith("/api/src_managed/main") for s in compile["sources"]) and not any(s.startswith("target/teq/") for s in compile["sources"])),
        ("api's description sources hold sbt's managed directory", any(s.endswith("/api/src_managed/main") for s in api["description"]["sources"])),
        ("api's test generator", test.get("generators") == [unnamed] and any(s.endswith("/api/src_managed/test") for s in test["sources"])),
        ("api's resource generator", compile.get("resourceGenerators") == [unnamed]),
        ("mirrored's resource generators the build sets", mirrored["configurations"]["compile"].get("resourceGenerators") == [unnamed]),
        ("api's test options", sorted(unsupported.get("test", [])) == ["Test / testOptions holds a Tests.Filter, a function the export cannot carry: name the suites to leave out in a Tests.Exclude", "Test / testOptions holds a Tests.Setup or Tests.Cleanup, a function the export cannot carry"]),
        ("api's stage, left out with its reasons", "stage" not in api and all(any(w in r for r in unsupported.get("stage", [])) for w in ("dockerGroupLayers", "scriptClasspath", "bashScriptTemplateLocation", "packageOptions"))),
    ]
    for name, block in before["projects"].items():
        if name == "mirrored":
            block = {**block, "configurations": {**block["configurations"], "compile": {**block["configurations"]["compile"], "resourceGenerators": [unnamed]}}}
        if name != "api":
            wanted.append((f"{name} as before", projects.get(name) == block))
    failed = [what for what, ok in wanted if not ok]
    for what in failed:
        print(f"not so: {what}")
    return not failed


if __name__ == "__main__":
    command, args = sys.argv[1], sys.argv[2:]
    commands = {"fixture": fixture, "header": header, "binaries": binaries, "pin": pin, "dependency": dependency, "alias": alias, "mirror": mirror, "classpaths": classpaths, "projects": projects, "configurations": scopes, "recorded": recorded}
    sys.exit(0 if commands[command](*args) else 1)
