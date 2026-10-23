"""SemanticDB's protobuf (semanticdb.proto, schema 4), decoded to a canonical form for comparison.

The field numbers and kinds are those of dotty 3.8.4's generated readers
(compiler/src/dotty/tools/dotc/semanticdb/generated/*.scala). A message decodes to a dict holding every field of
its schema, a scalar absent from the wire at its proto3 default, a plain message absent as None (dotty's
`Option[Scope]`, `Option[Range]`), every repeated field a list in wire order; a oneof message (Type, Signature,
Constant, Access, Tree) decodes to `{"kind": <case>, ...its fields}`, or `{"kind": "Empty"}` when no case is set or
the field is absent (dotty's typemappers read both as the Empty case); a float or a double is a canonical string
(`_ieee`), so NaN compares equal to NaN and a nonfinite value is no JSON number. Unknown fields, a wrong wire type, a truncated buffer or a length past the end are errors:
nothing is skipped, so a damaged document cannot compare equal to a whole one.
"""

import struct


class DecodeError(Exception):
    pass


# Field kinds: scalars by name; ("msg", Name) a message; ("rep", kind) a repeated field; ("enum", Name).
STRING, INT32, INT64, BOOL, FLOAT, DOUBLE = "string", "int32", "int64", "bool", "float", "double"

ENUMS = {
    "Schema": {0: "LEGACY", 3: "SEMANTICDB3", 4: "SEMANTICDB4"},
    "Language": {0: "UNKNOWN_LANGUAGE", 1: "SCALA", 2: "JAVA"},
    "Kind": {0: "UNKNOWN_KIND", 19: "LOCAL", 20: "FIELD", 3: "METHOD", 21: "CONSTRUCTOR", 6: "MACRO", 7: "TYPE",
             8: "PARAMETER", 17: "SELF_PARAMETER", 9: "TYPE_PARAMETER", 10: "OBJECT", 11: "PACKAGE",
             12: "PACKAGE_OBJECT", 13: "CLASS", 14: "TRAIT", 18: "INTERFACE"},
    "Role": {0: "UNKNOWN_ROLE", 1: "REFERENCE", 2: "DEFINITION"},
    "Severity": {0: "UNKNOWN_SEVERITY", 1: "ERROR", 2: "WARNING", 3: "INFORMATION", 4: "HINT"},
    "Format": {0: "HTML", 1: "MARKDOWN", 2: "JAVADOC", 3: "SCALADOC", 4: "KDOC"},
}

PROPERTIES = [(4, "ABSTRACT"), (8, "FINAL"), (16, "SEALED"), (32, "IMPLICIT"), (64, "LAZY"), (128, "CASE"),
              (256, "COVARIANT"), (512, "CONTRAVARIANT"), (1024, "VAL"), (2048, "VAR"), (4096, "STATIC"),
              (8192, "PRIMARY"), (16384, "ENUM"), (32768, "DEFAULT"), (65536, "GIVEN"), (131072, "INLINE"),
              (262144, "OPEN"), (524288, "TRANSPARENT"), (1048576, "INFIX"), (2097152, "OPAQUE")]

# Plain messages: field number -> (name, kind).
MESSAGES = {
    "TextDocuments": {1: ("documents", ("rep", ("msg", "TextDocument")))},
    "TextDocument": {1: ("schema", ("enum", "Schema")), 2: ("uri", STRING), 3: ("text", STRING),
                     11: ("md5", STRING), 10: ("language", ("enum", "Language")),
                     5: ("symbols", ("rep", ("msg", "SymbolInformation"))),
                     6: ("occurrences", ("rep", ("msg", "SymbolOccurrence"))),
                     7: ("diagnostics", ("rep", ("msg", "Diagnostic"))),
                     12: ("synthetics", ("rep", ("msg", "Synthetic")))},
    "SymbolInformation": {1: ("symbol", STRING), 16: ("language", ("enum", "Language")),
                          3: ("kind", ("enum", "Kind")), 4: ("properties", INT32), 5: ("display_name", STRING),
                          17: ("signature", ("msg", "Signature")),
                          13: ("annotations", ("rep", ("msg", "Annotation"))),
                          18: ("access", ("msg", "Access")),
                          19: ("overridden_symbols", ("rep", STRING)),
                          20: ("documentation", ("msg", "Documentation"))},
    "Annotation": {1: ("tpe", ("msg", "Type"))},
    "Documentation": {1: ("message", STRING), 2: ("format", ("enum", "Format"))},
    "Scope": {1: ("symlinks", ("rep", STRING)), 2: ("hardlinks", ("rep", ("msg", "SymbolInformation")))},
    "SymbolOccurrence": {1: ("range", ("msg", "Range")), 2: ("symbol", STRING), 3: ("role", ("enum", "Role"))},
    "Range": {1: ("start_line", INT32), 2: ("start_character", INT32), 3: ("end_line", INT32),
              4: ("end_character", INT32)},
    "Diagnostic": {1: ("range", ("msg", "Range")), 2: ("severity", ("enum", "Severity")), 3: ("message", STRING)},
    "Synthetic": {1: ("range", ("msg", "Range")), 2: ("tree", ("msg", "Tree"))},
    "CaseType": {1: ("key", ("msg", "Type")), 2: ("body", ("msg", "Type"))},
}

T, SC = ("msg", "Type"), ("msg", "Scope")
# Oneof messages: field number -> (case name, that case's fields).
ONEOFS = {
    "Type": {
        2: ("TypeRef", {1: ("prefix", T), 2: ("symbol", STRING), 3: ("type_arguments", ("rep", T))}),
        20: ("SingleType", {1: ("prefix", T), 2: ("symbol", STRING)}),
        21: ("ThisType", {1: ("symbol", STRING)}),
        22: ("SuperType", {1: ("prefix", T), 2: ("symbol", STRING)}),
        23: ("ConstantType", {1: ("constant", ("msg", "Constant"))}),
        17: ("IntersectionType", {1: ("types", ("rep", T))}),
        18: ("UnionType", {1: ("types", ("rep", T))}),
        19: ("WithType", {1: ("types", ("rep", T))}),
        7: ("StructuralType", {4: ("tpe", T), 5: ("declarations", SC)}),
        8: ("AnnotatedType", {3: ("annotations", ("rep", ("msg", "Annotation"))), 1: ("tpe", T)}),
        9: ("ExistentialType", {1: ("tpe", T), 3: ("declarations", SC)}),
        10: ("UniversalType", {3: ("type_parameters", SC), 2: ("tpe", T)}),
        13: ("ByNameType", {1: ("tpe", T)}),
        14: ("RepeatedType", {1: ("tpe", T)}),
        25: ("MatchType", {1: ("scrutinee", T), 2: ("cases", ("rep", ("msg", "CaseType")))}),
        26: ("LambdaType", {1: ("parameters", SC), 2: ("return_type", T)}),
    },
    "Signature": {
        1: ("ClassSignature", {1: ("type_parameters", SC), 2: ("parents", ("rep", T)), 3: ("self", T),
                               4: ("declarations", SC)}),
        2: ("MethodSignature", {1: ("type_parameters", SC), 2: ("parameter_lists", ("rep", SC)),
                                3: ("return_type", T)}),
        3: ("TypeSignature", {1: ("type_parameters", SC), 2: ("lower_bound", T), 3: ("upper_bound", T)}),
        4: ("ValueSignature", {1: ("tpe", T)}),
    },
    "Constant": {
        1: ("UnitConstant", {}), 2: ("BooleanConstant", {1: ("value", BOOL)}),
        3: ("ByteConstant", {1: ("value", INT32)}), 4: ("ShortConstant", {1: ("value", INT32)}),
        5: ("CharConstant", {1: ("value", INT32)}), 6: ("IntConstant", {1: ("value", INT32)}),
        7: ("LongConstant", {1: ("value", INT64)}), 8: ("FloatConstant", {1: ("value", FLOAT)}),
        9: ("DoubleConstant", {1: ("value", DOUBLE)}), 10: ("StringConstant", {1: ("value", STRING)}),
        11: ("NullConstant", {}),
    },
    "Access": {
        1: ("PrivateAccess", {}), 2: ("PrivateThisAccess", {}), 3: ("PrivateWithinAccess", {1: ("symbol", STRING)}),
        4: ("ProtectedAccess", {}), 5: ("ProtectedThisAccess", {}),
        6: ("ProtectedWithinAccess", {1: ("symbol", STRING)}), 7: ("PublicAccess", {}),
    },
    "Tree": {
        1: ("ApplyTree", {1: ("function", ("msg", "Tree")), 2: ("arguments", ("rep", ("msg", "Tree"))),
                          3: ("properties", INT32)}),
        2: ("FunctionTree", {1: ("parameters", ("rep", ("msg", "IdTree"))), 2: ("body", ("msg", "Tree"))}),
        3: ("IdTree", {1: ("symbol", STRING)}),
        4: ("LiteralTree", {1: ("constant", ("msg", "Constant"))}),
        5: ("MacroExpansionTree", {1: ("before_expansion", ("msg", "Tree")), 2: ("tpe", T)}),
        6: ("OriginalTree", {1: ("range", ("msg", "Range"))}),
        7: ("SelectTree", {1: ("qualifier", ("msg", "Tree")), 2: ("id", ("msg", "IdTree"))}),
        8: ("TypeApplyTree", {1: ("function", ("msg", "Tree")), 2: ("type_arguments", ("rep", T))}),
    },
}
# A message named by a oneof's case alone (FunctionTree's parameters are IdTrees).
MESSAGES["IdTree"] = {1: ("symbol", STRING)}


def _varint(b, i, end):
    r = s = 0
    while True:
        if i >= end:
            raise DecodeError("truncated varint")
        c = b[i]
        i += 1
        r |= (c & 0x7F) << s
        s += 7
        if c < 0x80:
            return r, i
        if s > 63:
            raise DecodeError("varint too long")


def _fields(b, i, end):
    while i < end:
        key, i = _varint(b, i, end)
        num, wire = key >> 3, key & 7
        if num == 0:
            raise DecodeError("field number 0")
        if wire == 0:
            v, i = _varint(b, i, end)
            yield num, wire, v
        elif wire == 1:
            if i + 8 > end:
                raise DecodeError("truncated fixed64")
            yield num, wire, b[i:i + 8]
            i += 8
        elif wire == 2:
            n, i = _varint(b, i, end)
            if i + n > end:
                raise DecodeError("length past the end")
            yield num, wire, (i, i + n)
            i += n
        elif wire == 5:
            if i + 4 > end:
                raise DecodeError("truncated fixed32")
            yield num, wire, b[i:i + 4]
            i += 4
        else:
            raise DecodeError(f"wire type {wire}")
    if i != end:
        raise DecodeError("message overruns its length")


def _signed(v, bits):
    v &= (1 << bits) - 1
    return v - (1 << bits) if v >> (bits - 1) else v


def _ieee(value, bits, canonical_nan, digits):
    """A float or double constant as a canonical string, so that two documents compare by the value's bits: a finite
    value as Python's shortest repr of it (`-0.0` apart from `0.0`), `Infinity`, `-Infinity`, `NaN` for the
    canonical quiet NaN (Java's `Float.NaN`/`Double.NaN`, what `0.0 / 0.0` folds to), another NaN with its bits."""
    if value != value:
        return "NaN" if bits == canonical_nan else "NaN(0x%0*X)" % (digits, bits)
    if value in (float("inf"), float("-inf")):
        return "Infinity" if value > 0 else "-Infinity"
    return repr(value)


def _default(kind):
    if kind == STRING:
        return ""
    if kind in (INT32, INT64):
        return 0
    if kind == BOOL:
        return False
    if kind in (FLOAT, DOUBLE):
        return "0.0"
    tag = kind[0]
    if tag == "rep":
        return []
    if tag == "enum":
        return ENUMS[kind[1]][0] if 0 in ENUMS[kind[1]] else 0
    if tag == "msg":
        # A oneof (Type, Signature, Constant, Access, Tree) absent is its Empty case, as dotty's typemappers read it;
        # a plain message absent (an unset Scope, Range, Documentation) is None, apart from a present empty one.
        return {"kind": "Empty"} if kind[1] in ONEOFS else None
    raise AssertionError(kind)


def _empty(name):
    """A message's fields at their defaults: what a present message holds before its fields are read."""
    if name in ONEOFS:
        return {"kind": "Empty"}
    return {fname: _default(fkind) for fname, fkind in MESSAGES[name].values()}


def _scalar(kind, wire, v, b):
    if kind == STRING:
        if wire != 2:
            raise DecodeError("string with wire type %d" % wire)
        try:
            return b[v[0]:v[1]].decode("utf-8")
        except UnicodeDecodeError as e:
            raise DecodeError(str(e))
    if kind in (INT32, INT64, BOOL) or (isinstance(kind, tuple) and kind[0] == "enum"):
        if wire != 0:
            raise DecodeError("varint field with wire type %d" % wire)
        if kind == INT32:
            return _signed(v, 32)
        if kind == INT64:
            return _signed(v, 64)
        if kind == BOOL:
            return v != 0
        table = ENUMS[kind[1]]
        return table.get(v, v)
    if kind == FLOAT:
        if wire != 5:
            raise DecodeError("float with wire type %d" % wire)
        return _ieee(struct.unpack("<f", v)[0], struct.unpack("<I", v)[0], 0x7FC00000, 8)
    if kind == DOUBLE:
        if wire != 1:
            raise DecodeError("double with wire type %d" % wire)
        return _ieee(struct.unpack("<d", v)[0], struct.unpack("<Q", v)[0], 0x7FF8000000000000, 16)
    raise AssertionError(kind)


def _decode_fields(schema, b, i, end, into):
    for num, wire, v in _fields(b, i, end):
        if num not in schema:
            raise DecodeError(f"unknown field {num}")
        fname, kind = schema[num]
        rep = isinstance(kind, tuple) and kind[0] == "rep"
        k = kind[1] if rep else kind
        if isinstance(k, tuple) and k[0] == "msg":
            if wire != 2:
                raise DecodeError(f"message field {fname} with wire type {wire}")
            val = decode_message(k[1], b, v[0], v[1])
        else:
            val = _scalar(k, wire, v, b)
        if rep:
            into[fname].append(val)
        else:
            into[fname] = val
    return into


def decode_message(name, b, i=0, end=None):
    end = len(b) if end is None else end
    if name in ONEOFS:
        cases = ONEOFS[name]
        out = {"kind": "Empty"}
        for num, wire, v in _fields(b, i, end):
            if num not in cases:
                raise DecodeError(f"unknown case {num} of {name}")
            if wire != 2:
                raise DecodeError(f"case {num} of {name} with wire type {wire}")
            if out["kind"] != "Empty":
                raise DecodeError(f"two cases of {name}")
            case, fields = cases[num]
            body = {fname: _default(fkind) for fname, fkind in fields.values()}
            _decode_fields(fields, b, v[0], v[1], body)
            out = {"kind": case, **body}
        return out
    return _decode_fields(MESSAGES[name], b, i, end, _empty(name))


def decode_documents(data):
    """The documents of a `.semanticdb` file (TextDocuments), each canonical."""
    return decode_message("TextDocuments", data)["documents"]


def properties_names(bits):
    """The names of a properties word's bits, for messages; an unknown bit is kept as a number."""
    names = [n for v, n in PROPERTIES if bits & v]
    rest = bits & ~sum(v for v, _ in PROPERTIES)
    return names + ([rest] if rest else [])


def diff(a, b, path="$"):
    """Every difference between two canonical values, as (path, left, right); lists compared in order."""
    out = []
    if isinstance(a, dict) and isinstance(b, dict):
        for k in sorted(set(a) | set(b), key=str):
            if k not in a or k not in b:
                out.append((f"{path}.{k}", a.get(k, "<absent>"), b.get(k, "<absent>")))
            else:
                out.extend(diff(a[k], b[k], f"{path}.{k}"))
    elif isinstance(a, list) and isinstance(b, list):
        for n, (x, y) in enumerate(zip(a, b)):
            out.extend(diff(x, y, f"{path}[{n}]"))
        if len(a) != len(b):
            out.append((f"{path}.length", len(a), len(b)))
    elif a != b or type(a) is not type(b):
        out.append((path, a, b))
    return out
