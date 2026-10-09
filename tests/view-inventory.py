#!/usr/bin/env python3
"""The inventory of the type-bearing records' reads and writes, and of the loader's boundary.

  python3 tests/view-inventory.py [--rev REV] [--sites] [--kind KIND]

Every use of a field of a symbol record that holds a type is found by the compiler, not by a
text search: the revision's tree (default HEAD) is copied, each field renamed, and `cargo check`
reports every place that names one, a read, a write, a construction or a pattern, with the
function it stands in. The fields: a signature (`SymInfo::sig`); a class's info (`parents`,
`base_types`, `underlying`, `declared_self`, `this_type`, the constructor's clauses `ctor`); a
type parameter's bounds (`upper`, `lower`); an alias's right-hand side and bounds (`rhs`,
`bounds`); the recorded expression types (`Program::expr_types`, stage 4's). A read is
classified by where it runs (the rules below, by file and function): a worker's read outside
the loader's lock, which the normalised accessors reconcile; the holder's under it, whose view
is the base; the interpreter's; after the join (the merge, the emitters, the pickler); the raw
traversal by design (the store, the measurement); a metadata-only read, which takes a base
type's class id alone; the accessors' own. A write by who makes it and where it becomes every
worker's. The loader's boundary is the callers of `with_loader`, `with_loader_for` and
`shared_write`, each with what crosses into the hold and out of it (a table kept here by hand,
checked against the callers the tree has).

The second pass finds the storage the peers' and the interpreter's
crossings go through the same way, a struct's field renamed within its struct or an enum's
variant within its enum: a `TClass`'s record ids (its methods, constructors, parameters, defaults,
initialisers, parent arguments, forwarders and bridges, which a peer reads its bodies by), a
quote's and a quote pattern's type (`TQuote::ty`, `TQuotePat::ty`), the typed patterns' types
(`TPat::Test`, `TPat::Class`), a stored inline body's types (`InlineDefinition`'s type, type
arguments, casts and node types, and the stored forms' copies, `StoredCopy`'s node types and
classes and `CopyMap`'s types), the interpreter's `Records` (the pending selections' type
arguments, the method and polymorphic types and the method types' ids), its type caches
(`lambda_of`, `val_type_refs`), `Value::Type` and the type tree `TreeRef::Type`. Their
transitive consumers (`CONSUMERS`, the quote builtins, the matcher, `tree_type`,
`instantiate_definition` and the rest) are listed with the uses the compiler found in each and
what each does with them at this revision, the record graphs a peer can reach (`ESCAPES`) with
the entry point a reader learns of each by and whether its owner writes it afterwards, and the
records that hold no type (`TYPE_FREE`: the parser's recovery records, the TASTy reader's
provenance), classified as type-free metadata (tables kept here by hand, their functions checked
against the tree and the type-free records' fields checked to name no type id).

--sites prints every site; --kind narrows it to one record kind (signature, class, tparam,
alias, expr, tclass, quote, pattern, inline, records, cache, value, tree). The check's own build
goes to out/view-inventory/.
"""
import argparse, collections, json, os, re, shutil, subprocess, sys, tempfile

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))

FIELDS = {
    'src/symbols.rs': {
        'parents': 'class', 'ctor': 'class', 'base_types': 'class', 'underlying': 'class', 'declared_self': 'class',
        'this_type': 'class', 'sig': 'signature', 'upper': 'tparam', 'lower': 'tparam', 'rhs': 'alias', 'bounds': 'alias',
    },
    'src/tir.rs': {'expr_types': 'expr'},
}
KINDS = ['signature', 'class', 'tparam', 'alias', 'expr']
KIND_NAMES = {
    'signature': "a symbol's signature", 'class': "a class's info", 'tparam': "a type parameter's bounds",
    'alias': "an alias's right-hand side and bounds", 'expr': 'the recorded expression types',
    'tclass': "a TClass's record ids", 'quote': "a quote's or quote pattern's type", 'pattern': "a typed pattern's type",
    'inline': "a stored inline body's types", 'records': "the interpreter's Records", 'cache': "the interpreter's type caches",
    'value': 'Value::Type', 'tree': 'TreeRef::Type',
}

# The second pass: fields renamed within their struct, variants within their enum.
STRUCT_FIELDS = [
    ('src/tir.rs', 'TClass', ['ctor_params', 'ctor_defaults', 'parent_args', 'parent_via', 'init', 'methods', 'ctors', 'forwarders', 'bridges'], 'tclass'),
    ('src/tir.rs', 'TQuote', ['ty'], 'quote'),
    ('src/tir.rs', 'TQuotePat', ['ty'], 'quote'),
    ('src/tir.rs', 'InlineDefinition', ['ty', 'type_args', 'casts', 'node_types'], 'inline'),
    ('src/typer/quoted.rs', 'StoredCopy', ['types', 'classes'], 'inline'),
    ('src/typer/quoted.rs', 'CopyMap', ['types'], 'inline'),
    ('src/interp/mod.rs', 'Records', ['pending', 'method_types', 'method_type_ids', 'poly_types'], 'records'),
    ('src/interp/mod.rs', 'InterpCaches', ['lambda_of'], 'cache'),
    ('src/interp/mod.rs', 'Interp', ['lambda_of', 'val_type_refs'], 'cache'),
]
VARIANTS = [
    ('src/tir.rs', 'TPat', ['Test', 'Class'], 'pattern'),
    ('src/interp/value.rs', 'Value', ['Type'], 'value'),
    ('src/interp/value.rs', 'TreeRef', ['Type'], 'tree'),
]
KINDS2 = ['tclass', 'quote', 'pattern', 'inline', 'records', 'cache', 'value', 'tree']

# The transitive consumers of that storage and of the record kinds above that the peers' and the
# interpreter's crossings reach, with what each does with the types it reads at this revision
# (kept by hand; `main` checks that each function exists).
CONSUMERS = {
    'install_quotes': "`$quote` and `$quoteMatch` take a TQuote's or a pattern's type (the record a peer's or the base's) into the reader's view before they substitute (`TypeStore::in_view_here`)",
    'clone_local': "copies a local's signature into a fresh local: through `Symbols::sym`, the views (a peer's binder's signature imported)",
    'subst_local_type': "rewrites the fresh local's signature, its own record, before the tree that names it escapes",
    'pat': "`Matcher::pat` sends a typed pattern's types (the quote pattern's record) into `unify`",
    'unify': "takes the pattern's type and the scrutinee's into the reader's view (`in_view_here`) and compares two ids of one view",
    'tree_type': "the type of a tree a macro holds: the recorded expression type through `Program::type_of`, read after its expression's escape mark and translated into the reader's view while the table is shared (`expr_type_in_view`)",
    'val_type_ref': "tells a `Symbol.typeRef` by the store's provenance (`TypeStore::is_unshared`), which survives an import",
    'instantiate_definition': "rewrites a stored body's type parameters' bounds, binders' signatures, type arguments and casts through `demand_type`",
    'demand_type': "takes each type into the reader's view (`in_view_here`) before the substitution, the paths and `C.this`",
    'recorded': "a stored body's node type (`StoredCopy::types`, its record's: a peer's or the base's) or the program's, which the copy's `ty_under` imports before the substitution",
    'ty_under': "imports the type into the reader's view (`TypeStore::import`) before the substitution, the classes' copies and the paths",
    'stored_graph': "the stored classes' ids (`StoredCopy::classes`): class ids alone, no type",
    'overrides_by_signature': "reads two symbols' signatures and their type parameters' bounds: through `Symbols::sig` and `tparam`, the views",
    'alias_body': "reads an alias's right-hand side: through `Symbols::alias`, the views",
    'candidate_type_has_vars': "consumes the normalised signature it asks for (`sig_arc`)",
    'compute_coerce': "imports a loader-typed body's recorded type (stage 2)",
    'deferred_body': "types a library or std body under the loader's lock for the interpreter: the holder's",
    'scala_mirror_value_unlocked': "translates the declared type into the base at entry (stage 2); reads the class's own records under the hold through the view's export",
    'check_class_for_run': "checks a program class a macro's run met: under the hold when the run is the holder's, its records exported",
    'restore': "puts back the caches the run took, the set of the view the run is in (`take_caches`, the worker's or the holder's)",
}

# The records the stages since the second pass added that hold no type, listed so that a reader
# finds them classified rather than left out: each is type-free metadata (kept by hand; `main`
# checks that the struct exists and that its fields name no `TypeId`).
TYPE_FREE = [
    ('src/ast.rs', 'Recovery', "the parser's recovery records: a site and the span of the tokens skipped, read by the typer as positions"),
    ('src/tasty/origins.rs', 'Origins', "the TASTy reader's provenance of a product file: its key, its token, its definitions' addresses and their names' offsets"),
    ('src/typer/loader/provenance.rs', 'Tokens', "the provenance's tokens seen and the sources' identities holding each, names alone"),
]

# The record graphs a peer can reach, with the entry point that gives a reader the id and what
# the owner may still write after it (kept by hand; the functions are checked against the tree).
ESCAPES = [
    ("a program symbol's signature completed in the body phase and its type parameters, the owner's records", 'publish_sig',
     "the signature's cell done at the release of the hold that exported it; the type parameters' ids in its types",
     "nothing: the bounds are resolved before the publication (`resolve_tparam_bounds`)"),
    ("a partial signature of a recursive definition", 'set_partial_sig',
     "the partial signature at the release of its hold", "the complete signature replaces it (`publish_sig`): a new version"),
    ("a body typed into the owner's chunk: its TFun, trees, patterns, locals, local classes, quotes, expression types", 'publish_body',
     "`fun_of_sym` and `val_init` at the release; the body cell done after", "an expression's type set again through `typed_forked` where `writable` holds"),
    ("a class checked into the owner's chunk: its TClass and what it names", 'publish_class_bodies',
     "`class_bodies` and `class_done` at the release", "methods reached late go to `late_methods`, applied after the merge"),
    ("a quote's binders cloned for a run", 'clone_local',
     "the tree the run builds, published with the body it lands in", "`subst_local_type` replaces the clone's signature after it is marked done, before the tree escapes"),
    ("a stored inline body's instance", 'instantiate_definition',
     "the expansion's tree, published with the body it lands in", "the fresh type parameters' bounds and the binders' signatures, before the tree escapes"),
]

# Where a read runs, by file (a prefix) and then by function.
AFTER_JOIN_FILES = ('src/emit/', 'src/jvm/', 'src/tasty/', 'src/held.rs', 'src/tir.rs', 'src/typer/merge.rs', 'src/typer/incremental.rs')
BY_DESIGN_FILES = ('src/types.rs', 'src/types/', 'src/typer/measured.rs', 'src/typer/profile.rs')
HOLDER_FILES = ('src/typer/loader/', 'src/typer/arity.rs', 'src/typer/stdlib.rs')
INTERP_FILES = ('src/interp/',)
ACCESSOR_FILES = ('src/symbols.rs',)
# Functions that run under the loader's lock wherever they stand: the `_unlocked` halves of the
# loader's operations and what only they call.
HOLDER_FNS = re.compile(r'_unlocked$|^(link_library_class|complete_java_class|enter_java_class|set_loaded_parents|set_tparam_bounds|complete_loaded_alias|loaded_sig)$')
# A read of a class's base types that keeps the class id and drops the type.
METADATA = re.compile(r'base_types(\[[^\]]*\])?\.0\b|base_types(\.iter\(\)|\.get\(\w+\))(\.skip\(\d+\)|\.rev\(\))*\.(map|any|all|find|position|filter)\(\|&{0,2}\(\w+, _\)\||\(\w+, _\) in [\w.()&]*base_types|base_types\.len\(\)|base_types\.is_empty\(\)|base_types\.first\(\)\.map\(\|&?\(\w+, _\)\|')
# ... unless the same line takes the type too.
USES_TYPE = re.compile(r'\|&{0,2}\(_, \w+\)\||base_types(\[[^\]]*\])?\.1\b|\(\w+, \w+\) in [\w.()&]*base_types|let \(\w+, \w+\) = ')

# The loader's boundary: per operation run under the lock, what the worker hands in and what it
# takes out. Ids alone cross unless the entry says otherwise; `types` is the case the
# canonicalisation rule covers (translated into the base at entry, the results read back through
# the accessors).
LOADER = {
    'scala_mirror_value_unlocked': ('the declared type of the mirror (a type, translated into the base at entry)', 'a tree over a new shared symbol whose signature is the declared type'),
    'merged_overload_unlocked': ('the two owners (types, translated into the base at entry), their members\' signatures (`parameters_of`, the holder\'s view)', 'the merged symbol'),
    'bind_mixin_supers_unlocked': ('ids', 'nothing'),
    'resolve_declaration': ('ids (a library body\'s declaration reference)', 'the declaration\'s resolution (and for the declarations\' listing its owner): ids'),
    'std_body_unlocked': ('a symbol', 'nothing (the body published with its cell)'),
    'complete_sig_inner': ('a symbol', 'its signature, read back through the accessor (`uncompleted_sig`)'),
    'uncompleted_sig': ('a symbol', 'its signature, read back through the accessor'),
    'loaded_sig': ('a symbol', 'its signature, published and returned (`complete_sig`)'),
    'publish_sig': ('the signature (types, exported under the hold, `published_sig`)', 'nothing'),
    'set_partial_sig': ('the partial signature (types, exported under the hold, `published_sig`)', 'nothing'),
    'type_member_body': ('a symbol', 'nothing (the body published with its cell)'),
    'publish_body': ('ids', 'nothing'),
    'publish_class_bodies': ('the pending variance registrations (types, exported under the hold) and templates', 'nothing'),
    'class_done': ('ids', 'nothing'),
    'erased_tags': ('a symbol and bits', 'nothing'),
    'mods': ('a symbol and its flags', 'nothing'),
    'push_override': ('two symbols', 'nothing'),
    'overridden_by_param': ('a symbol', 'nothing'),
    'needs_accessor': ('a symbol', 'nothing'),
    'derived_opaque_unlocked': ('a class and a member', 'the derived symbol'),
    'import_hidden': ('names', 'a list of names'),
    'import_values': ('a value import (a symbol)', 'its index'),
    'complete_alias_now': ('an alias', 'nothing (read back through the accessor)'),
    'complete_class_uncompleted': ('a class', 'nothing (read back through the accessor)'),
    'complete_class_now_unlocked': ('a class', 'nothing (read back through the accessor)'),
    'children': ('two classes', 'nothing'),
    'subclasses': ('two classes', 'nothing'),
    'outer_accessor_unlocked': ('a class', 'a symbol'),
    'define_outer_accessors_unlocked': ('a class', 'nothing'),
    'outer_this_sym_unlocked': ('a class', 'a symbol'),
    'by_name_class_unlocked': ('nothing', 'a class'),
    'repeated_class_unlocked': ('nothing', 'a class'),
    'synthesize_tuple_unlocked': ('an arity', 'a class'),
    'synthesize_function_unlocked': ('an arity and a context flag', 'a class'),
    'census': ('text', 'nothing'),
    'enter_std_for_unlocked': ('a package and a name', 'an entry'),
    'demand_std_givens_unlocked': ('a package', 'nothing'),
    'unlock_std_package_unlocked': ('a package and a name', 'nothing'),
    'enter_std_slot_unlocked': ('a slot', 'nothing'),
    'any_member_of_unlocked': ('a trait and a name', 'a symbol'),
    'settle_merged': ('a class, an entry and its alternatives (symbols)', 'the settled entry'),
    'merge_inherited_unlocked': ('a class and an entry', 'the settled entry'),
    'dispatch_name': ('a symbol and a name', 'nothing'),
    'convert_library_class_unlocked': ('a class', 'nothing'),
    'check_library_class_unlocked': ('a class', 'nothing'),
    'name_library_members_unlocked': ('a class', 'nothing'),
    'library_body_unlocked': ('a symbol', 'nothing'),
    'absorb_java_ctors_unlocked': ('a class', 'nothing'),
    'absorb_java_class_unlocked': ('a class', 'whether it was absorbed'),
    'enter_java_builtin_members_unlocked': ('a class', 'whether they were entered'),
    'load_pkg_member_unlocked': ('a package and a name', 'whether it was entered'),
    'java_class_file_named_unlocked': ('a class path slot and a name', 'the class file'),
    'enter_pkg_objects_unlocked': ('a package', 'whether they were entered'),
    'check_class_for_run': ('a class', 'nothing (its `TClass` read by id)'),
    'derived_match_alias': ('a shared match alias, the prefix and the substitution (types, exported under the hold)', "the alias's copy, a shared record read through the accessor"),
}


def copy_tree(rev, dest):
    archive = subprocess.run(['git', '-C', ROOT, 'archive', rev], check=True, capture_output=True).stdout
    subprocess.run(['tar', '-x', '-C', dest], input=archive, check=True)


def rename_fields(dest):
    for path, fields in FIELDS.items():
        full = os.path.join(dest, path)
        text = open(full).read()
        for f in fields:
            new, n = re.subn(r'(\n    pub )' + f + r'(: )', r'\g<1>' + f + r'__view_inventory\2', text, count=1)
            if n != 1:
                sys.exit(f'view-inventory: no field `{f}` in {path}')
            text = new
        open(full, 'w').write(text)


def rename_second(dest):
    """The second pass's renames: each field within its struct, each variant within its enum."""
    names = {}
    specs = [(p, r'\n(?:pub(?:\([\w:]+\))? )?struct ' + o + r'\b[^{;]*\{', o, fs, r'(\n    (?:pub(?:\([\w:]+\))? )?)', r'(: )', k) for p, o, fs, k in STRUCT_FIELDS]
    specs += [(p, r'\npub enum ' + o + r'\b[^{]*\{', o, vs, r'(\n    )', r'(\()', k) for p, o, vs, k in VARIANTS]
    for path, head, owner, members, before, after, kind in specs:
        full = os.path.join(dest, path)
        text = open(full).read()
        m = re.search(head, text)
        if not m:
            sys.exit(f'view-inventory: no `{owner}` in {path}')
        end = text.index('\n}', m.end())
        block = text[m.end():end]
        for f in members:
            block, n = re.subn(before + f + after, r'\g<1>' + f + r'__view_inventory\g<2>', block, count=1)
            if n != 1:
                sys.exit(f'view-inventory: no `{f}` in `{owner}` ({path})')
            names[(owner, f)] = kind
        open(full, 'w').write(text[:m.end()] + block + text[end:])
    return names


def second_errors(dest, names):
    """The uses of the second pass's names, each with its owner and kind: the name the message
    quotes and the struct or enum it names beside it (the one owner of that name otherwise)."""
    target = os.path.join(ROOT, 'out', 'view-inventory')
    env = dict(os.environ, CARGO_TARGET_DIR=target)
    r = subprocess.run(['cargo', 'check', '--message-format=json'], cwd=dest, env=env, capture_output=True, text=True, timeout=340)
    owners = {}
    for (o, f), k in names.items():
        owners.setdefault(f, {})[o] = k
    out = []
    for line in r.stdout.splitlines():
        try:
            m = json.loads(line)
        except json.JSONDecodeError:
            continue
        if m.get('reason') != 'compiler-message' or m['message']['level'] != 'error':
            continue
        msg = m['message']
        quoted = re.findall(r'`([^`]+)`', msg['message'])
        spans = [s for s in msg['spans'] if s['is_primary']]
        if not spans:
            continue
        hit = None
        for q in quoted:
            kinds = owners.get(q.split('::')[-1].split('(')[0])
            if not kinds:
                continue
            owner = next((o for o in kinds if any(re.search(r'\b' + o + r'\b', x) for x in quoted)), None)
            if owner is None and len(kinds) == 1:
                owner = next(iter(kinds))
            if owner is not None:
                hit = (owner, q.split('::')[-1].split('(')[0], kinds[owner])
                break
        if hit is None:
            continue
        s = spans[0]
        out.append((s['file_name'], s['line_start'], s['column_start'], hit, msg['message']))
    if not out:
        sys.exit("view-inventory: the second pass's check reported no use:\n" + r.stderr[-2000:])
    return out


def second_pass(rev):
    """The second pass's sites, each with its kind, function, access and where it runs."""
    with tempfile.TemporaryDirectory(prefix='teq-view-inventory-') as dest:
        copy_tree(rev, dest)
        missing = [f for f in list(CONSUMERS) + [e[1] for e in ESCAPES] if not function_exists(dest, f)]
        if missing:
            sys.exit('view-inventory: no function ' + ', '.join(missing) + ' in the tree')
        free = type_free(dest)
        names = rename_second(dest)
        src = Source(dest)
        sites = []
        for f, line, col, (owner, name, kind), message in second_errors(dest, names):
            text = src.lines(f)[line - 1]
            site = dict(file=f, line=line, field=f'{owner}::{name}', kind=kind, fn=src.function(f, line), text=text.strip())
            site['access'] = 'use' if kind in ('pattern', 'value', 'tree') else access(text[col - 1:], name, message)
            site['class'] = classify_read(site) if site['access'] in ('read', 'use') else classify_write(site)
            sites.append(site)
    return sites, free


def type_free(dest):
    """Each struct of TYPE_FREE with its description, refused where a field names a type id."""
    out = []
    for path, name, what in TYPE_FREE:
        text = open(os.path.join(dest, path)).read()
        m = re.search(r'\n(?:pub(?:\([\w:]+\))? )?struct ' + name + r'\b[^{;]*\{', text)
        if not m:
            sys.exit(f'view-inventory: no `{name}` in {path}')
        body = text[m.end():text.index('\n}', m.end())]
        if re.search(r'\bTypeId\b', body):
            sys.exit(f'view-inventory: `{name}` ({path}) holds a type id: not type-free metadata')
        out.append((path, name, what))
    return out


def function_exists(dest, name):
    for dirpath, _, files in os.walk(os.path.join(dest, 'src')):
        for n in files:
            if n.endswith('.rs') and re.search(r'\bfn ' + re.escape(name) + r'\b', open(os.path.join(dirpath, n)).read()):
                return True
    return False


def compile_errors(dest):
    target = os.path.join(ROOT, 'out', 'view-inventory')
    env = dict(os.environ, CARGO_TARGET_DIR=target)
    r = subprocess.run(['cargo', 'check', '--message-format=json'], cwd=dest, env=env, capture_output=True, text=True, timeout=340)
    out = []
    for line in r.stdout.splitlines():
        try:
            m = json.loads(line)
        except json.JSONDecodeError:
            continue
        if m.get('reason') != 'compiler-message' or m['message']['level'] != 'error':
            continue
        msg = m['message']
        named = [f for f in re.findall(r'`(\w+?)(?:__view_inventory)?`', msg['message']) if any(f in fs for fs in FIELDS.values())]
        spans = [s for s in msg['spans'] if s['is_primary']]
        if not named or not spans:
            continue
        s = spans[0]
        out.append((s['file_name'], s['line_start'], s['column_start'], named[-1], msg['message']))
    if not out:
        sys.exit('view-inventory: the check reported no use of the fields:\n' + r.stderr[-2000:])
    return out


class Source:
    def __init__(self, dest):
        self.dest, self.files = dest, {}

    def lines(self, f):
        if f not in self.files:
            self.files[f] = open(os.path.join(self.dest, f)).read().split('\n')
        return self.files[f]

    def function(self, f, line):
        for text in reversed(self.lines(f)[:line]):
            m = re.search(r'\bfn\s+(\w+)', text)
            if m and not text.lstrip().startswith('//'):
                return m.group(1)
        return '?'


def access(text, field, message):
    if 'has no field named' in message or 'does not have' in message or 'pattern' in message:
        return 'construct'
    m = re.search(r'\b' + field + r'\b(.*)', text)
    after = m.group(1) if m else ''
    if re.match(r'(\[[^\]]*\])?(\.\d)?\s*(=[^=]|\+=|-=|\|=|&=)', after):
        return 'write'
    if re.match(r'\s*\.(push|insert|retain|extend|clear|truncate|sort|dedup|iter_mut|get_mut|as_mut|take|drain|remove|swap)\b', after):
        return 'write'
    return 'read'


def classify_read(site):
    f, fn, text = site['file'], site['fn'], site['text']
    if re.search(r'\bclass_raw\(|\.raw\(', text):
        return 'metadata only'
    if f.startswith(ACCESSOR_FILES):
        return 'the accessors'
    if f.startswith(BY_DESIGN_FILES):
        return 'raw by design'
    if f.startswith(AFTER_JOIN_FILES):
        return 'after the join'
    if site['field'] == 'base_types' and METADATA.search(text) and not USES_TYPE.search(text):
        return 'metadata only'
    if f.startswith(HOLDER_FILES) or HOLDER_FNS.search(fn):
        return 'the holder'
    if f.startswith(INTERP_FILES):
        return 'the interpreter'
    return 'a worker'


READ_CLASSES = ['a worker', 'the holder', 'the interpreter', 'metadata only', 'after the join', 'raw by design', 'the accessors']


def classify_write(site):
    f, fn = site['file'], site['fn']
    if f.startswith(('src/typer/merge.rs', 'src/typer/incremental.rs', 'src/held.rs')):
        return 'after the join'
    if f.startswith('src/typer/namer.rs') or fn in ('new_class', 'new_sym', 'new_tparam', 'new_overloaded', 'new', 'enter_def', 'default', 'attach'):
        return 'a new record'
    if fn in ('publish_sig', 'set_partial_sig', 'scala_mirror_value_unlocked', 'publish_class_bodies'):
        return "a worker's publication under the hold"
    if f.startswith(HOLDER_FILES) or HOLDER_FNS.search(fn) or fn in ('mirror_val_sym', 'complete_class_inner', 'complete_class_def', 'set_parents', 'set_opaque_bound', 'resolve_tparam_bounds_of', 'finish_alias', 'check_member_cycle', 'seal_primary_ctor', 'resolve_clauses', 'enter_value_import', 'member_in_class', 'derived_match_alias'):
        return "the holder's completion (a shared record's) or a worker's own record"
    return "a worker's own record"


def loader_sites(src_root):
    out = []
    pat = re.compile(r'\b(with_loader|with_loader_for|shared_write)\(')
    for dirpath, _, names in os.walk(os.path.join(src_root, 'src')):
        for n in sorted(names):
            if not n.endswith('.rs'):
                continue
            path = os.path.join(dirpath, n)
            rel = os.path.relpath(path, src_root)
            lines = open(path).read().split('\n')
            for i, text in enumerate(lines):
                if not pat.search(text) or text.lstrip().startswith('//') or re.search(r'\bfn (with_loader|shared_write)', text):
                    continue
                window = ' '.join(lines[i:i + 12])
                body = window[pat.search(window).end():]
                if re.match(r'\s*f\)', body) or 'with_loader_locked' in text:
                    continue
                patterns = [(name, r'\b' + name + r'\b') for name in LOADER] + [
                    ('census', r'loaded_mut\(\)'), ('dispatch_name', r'dispatch_names'),
                    ('publish_sig', r'sym_mut\(sym\)\.sig = Some\(sig\)'), ('set_partial_sig', r'sig = Some\(partial\)'),
                ]
                found = [(m.start(), name) for name, p in patterns for m in [re.search(p, body[:600])] if m]
                callee = min(found)[1] if found else None
                caller = None
                for t in reversed(lines[:i + 1]):
                    m = re.search(r'\bfn\s+(\w+)', t)
                    if m:
                        caller = m.group(1)
                        break
                out.append((rel, i + 1, caller, callee))
    return out


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--rev', default='HEAD')
    ap.add_argument('--sites', action='store_true')
    ap.add_argument('--kind', choices=KINDS + KINDS2)
    args = ap.parse_args()
    with tempfile.TemporaryDirectory(prefix='teq-view-inventory-') as dest:
        copy_tree(args.rev, dest)
        loaders = loader_sites(dest)
        rename_fields(dest)
        errors = compile_errors(dest)
        src = Source(dest)
        kind_of = {f: k for fs in FIELDS.values() for f, k in fs.items()}
        sites = []
        for f, line, col, field, message in errors:
            text = src.lines(f)[line - 1]
            site = dict(file=f, line=line, field=field, kind=kind_of[field], fn=src.function(f, line), text=text.strip())
            site['access'] = access(text[col - 1:], field, message)
            site['class'] = classify_read(site) if site['access'] == 'read' else classify_write(site)
            sites.append(site)
    rev = subprocess.run(['git', '-C', ROOT, 'rev-parse', '--short', args.rev], capture_output=True, text=True).stdout.strip()
    print(f'view-inventory: {rev}, {len(sites)} uses of the type-bearing fields, found by the compiler')
    print()
    print('The reads, by record kind and where they run')
    reads = collections.Counter((s['kind'], s['class']) for s in sites if s['access'] == 'read')
    print(f"{'':44}" + ''.join(f'{c:>16}' for c in READ_CLASSES) + f"{'all':>8}")
    for k in KINDS:
        row = [reads[(k, c)] for c in READ_CLASSES]
        print(f'{KIND_NAMES[k]:44}' + ''.join(f'{n:>16}' for n in row) + f'{sum(row):>8}')
    print()
    print('The reads of a worker, by file')
    by_file = collections.Counter((s['file'], s['kind']) for s in sites if s['access'] == 'read' and s['class'] == 'a worker')
    files = sorted({f for f, _ in by_file}, key=lambda f: -sum(by_file[(f, k)] for k in KINDS))
    for f in files:
        print(f'  {f:40}' + ', '.join(f'{by_file[(f, k)]} {k}' for k in KINDS if by_file[(f, k)]))
    print()
    print('The writes and constructions, by record kind and who makes them')
    writes = collections.Counter((s['kind'], s['class']) for s in sites if s['access'] != 'read')
    for k in KINDS:
        for c in sorted({c for kk, c in writes if kk == k}):
            print(f'  {KIND_NAMES[k]:44} {c:70} {writes[(k, c)]:>4}')
    print()
    print("The loader's boundary: the callers of with_loader, with_loader_for and shared_write")
    for rel, line, caller, callee in loaders:
        what = LOADER.get(callee, ('?', '?'))
        print(f'  {rel}:{line} {caller} -> {callee or "?"}: in {what[0]}; out {what[1]}')
    unknown = [l for l in loaders if l[3] is None]
    if unknown:
        print(f'view-inventory: {len(unknown)} loader sites not in the table', file=sys.stderr)
    second, free = second_pass(args.rev)
    print()
    print("The second pass: the storage the peers' and the interpreter's crossings go through, found by the compiler")
    print(f"{'':44}" + ''.join(f'{c:>16}' for c in READ_CLASSES + ['writes']) + f"{'all':>8}")
    for k in KINDS2:
        row = [sum(1 for s in second if s['kind'] == k and s['access'] in ('read', 'use') and s['class'] == c) for c in READ_CLASSES]
        writes = sum(1 for s in second if s['kind'] == k and s['access'] not in ('read', 'use'))
        print(f'{KIND_NAMES[k]:44}' + ''.join(f'{n:>16}' for n in row + [writes]) + f'{sum(row) + writes:>8}')
    print()
    print('The transitive consumers: the uses the compiler found in each (both passes), and what each does with them')
    every = sites + second
    for fn, what in CONSUMERS.items():
        uses = collections.Counter(s['kind'] for s in every if s['fn'] == fn)
        found = ', '.join(f'{n} {k}' for k, n in sorted(uses.items())) or 'no field directly'
        print(f'  {fn}: {found}; {what}')
    print()
    print('The record graphs a peer can reach: the entry point a reader learns of each by, and what the owner writes after it')
    for graph, fn, entry, after in ESCAPES:
        print(f'  {graph} ({fn}): learnt of through {entry}; after it, {after}')
    print()
    print('The records that hold no type, type-free metadata: no crossing reaches them')
    for path, name, what in free:
        print(f'  {name} ({path}): {what}')
    if args.sites:
        print()
        print('Every site')
        for s in sites + second:
            if args.kind and s['kind'] != args.kind:
                continue
            print(f"  {s['file']}:{s['line']} {s['fn']} [{s['kind']}, {s['access']}, {s['class']}] {s['text'][:120]}")


if __name__ == '__main__':
    main()
