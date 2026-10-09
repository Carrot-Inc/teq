"""The shared module: the model types with their codecs and schemas, the endpoint definitions
over them, and the sample values the self-tests of both sides use."""
import os

from . import vocab
from .emit import render_decl, sample_value
from .universe import BLOCKED_TYPES, CASE_BUCKETS, FIELD_BUCKETS, PRIMS, Decl, Ty, list_of, needs_of, option_of

MODEL_HEADER = """package meridian.model

import meridian.core.*
import meridian.core.Codecs.given
import cats.syntax.all.*
{extra}
"""

SAMPLES_HEADER = """package meridian.model.samples

import meridian.core.*
import meridian.model.*
{extra}
"""

ROUTES_HEADER = """package meridian.model.api

import meridian.core.*
import meridian.core.Codecs.given
import meridian.core.http.Credentials
import meridian.model.*
{extra}
"""

# The derives clauses of the surveyed model module by kind of declaration, with the survey's
# type classes named as the corpus names them (Enumerated, Labelled, the id wrappers).
CLASS_SETS = [(["JsonCodec", "Schema"], 470), (["Eq", "JsonCodec", "Schema"], 33), (["JsonCodec"], 14), (["Eq", "Schema"], 9), (["JsonCodec", "Monoid", "Schema"], 6),
              (["JsonCodec", "Schema", "Show"], 4), (["Eq"], 3), (["Eq", "JsonCodec", "Schema", "Show"], 3), (["JsonCodec", "Show"], 3), (["Schema"], 3), (["Eq", "JsonCodec"], 1)]
ENUM_SETS = [(["Enumerated"], 54), (["Enumerated", "Eq"], 44), (["Enumerated", "Eq", "JsonCodec", "Schema", "Show"], 5), (["Enumerated", "Eq", "Show"], 4),
             (["Enumerated", "Eq", "JsonCodec", "Schema"], 3), (["Enumerated", "JsonCodec", "Schema"], 3), (["Enumerated", "Order"], 3), (["Enumerated", "Eq", "Order"], 1),
             (["Enumerated", "JsonCodec", "Schema", "Show"], 1), (["Enumerated", "Schema"], 1), (["Enumerated", "Show"], 1)]
ADT_SETS = [(["JsonCodec", "Schema"], 60), (["Eq", "JsonCodec", "Schema"], 10), (["Eq", "Labelled", "JsonCodec", "Schema"], 6), (["Labelled"], 1), (["Labelled", "JsonCodec", "Schema"], 1), (["JsonCodec", "Schema", "Show"], 2)]


class EndpointRec:
    def __init__(self, area, obj, name, method, segments, captures, queries, headers, body, output, status, action, entity, kind):
        self.area = area
        self.obj = obj
        self.name = name
        self.method = method
        self.segments = segments
        self.captures = captures
        self.queries = queries
        self.headers = headers
        self.body = body
        self.output = output
        self.status = status
        self.action = action
        self.entity = entity
        self.kind = kind

    def inputs(self):
        items = [t for _, t in self.captures] + [t for _, t in self.queries] + [t for _, t in self.headers]
        if self.body is not None:
            items.append(self.body)
        return items

    def input_type(self):
        items = self.inputs()
        if not items:
            return "Unit"
        if len(items) == 1:
            return items[0].expr
        return "(" + ", ".join(t.expr for t in items) + ")"

    def input_sample(self, rng):
        items = self.inputs()
        if not items:
            return "()"
        if len(items) == 1:
            return items[0].sample(rng, 1)
        return "(" + ", ".join(t.sample(rng, 1) for t in items) + ")"

    def output_type(self):
        return self.output.expr if self.output is not None else "Unit"

    def ref(self):
        return f"{self.obj}.{self.name}"

    def path_text(self):
        parts = []
        for kind, value in self.segments:
            parts.append(value if kind == "fixed" else "{" + value + "}")
        return "/".join(parts)


def scaled(count, scale):
    return max(1, round(count * scale))


def make_entity_members(decl, rng, enums):
    status = [(n, t) for n, t in decl.fields if t.kind == "enum" and t.eq]
    members = []
    if status:
        name, t = status[0]
        cases = next(d for d in enums if d.name == t.expr).cases
        members.append(f"def is{cases[0]}: Boolean = {name} === {t.expr}.{cases[0]}")
        members.append(f"def with{name[0].upper() + name[1:]}(value: {t.expr}): {decl.name} = copy({name} = value)")
    texts = [n for n, t in decl.fields if t.expr == "String"]
    if texts:
        members.append(f"def {texts[0]}Slug: String = Text.slug({texts[0]})")
    ints = [n for n, t in decl.fields if t.expr in ("Int", "Long")]
    if len(ints) >= 2:
        members.append(f"def {ints[0]}Share: Long = if {ints[1]} == 0 then 0L else {ints[0]}.toLong * 100L / {ints[1]}.toLong")
    options = [n for n, t in decl.fields if t.kind == "option"]
    if options:
        members.append(f"def has{options[0][0].upper() + options[0][1:]}: Boolean = {options[0]}.isDefined")
    lists = [n for n, t in decl.fields if t.kind == "list"]
    if lists:
        members.append(f"def {lists[0]}Count: Int = {lists[0]}.length")
    return members[: rng.randint(1, 4)]


def write_shared(u, out, targets, rng, flags, scale):
    """Writes the model and api packages; returns (endpoint records, areas)."""
    model_dir = os.path.join(out, "shared", "model")
    api_dir = os.path.join(out, "shared", "model", "api")
    samples_dir = os.path.join(out, "shared", "model", "samples")
    for d in (model_dir, api_dir, samples_dir):
        os.makedirs(d, exist_ok=True)

    n_classes = scaled(targets["case_classes"]["count"], scale)
    n_enums = scaled(targets["enums"]["count"], scale)
    n_ids = scaled(76, scale)
    n_areas = max(2, scaled(56, scale))
    n_wrappers = scaled(40, scale)
    class_fields = targets["case_classes"]["fields"]
    enum_cases = targets["enums"]["cases"]

    class_sets = expand_sets(CLASS_SETS, n_classes - n_wrappers - n_ids, rng)
    singleton_share = 0.58
    n_singletons = round(n_enums * singleton_share)
    enum_sets = expand_sets(ENUM_SETS, n_singletons, rng)
    adt_sets = expand_sets(ADT_SETS, n_enums - n_singletons, rng)

    area_names = []
    while len(area_names) < n_areas:
        name = vocab.camel([rng.pick(vocab.NOUNS)])
        if name not in area_names and name not in u.names.used and name not in BLOCKED_TYPES:
            area_names.append(name)
            u.names.used.add(name)
    per_area = distribute(n_classes - n_wrappers - n_ids, n_areas, rng, 2)
    enums_per_area = distribute(n_singletons, n_areas, rng, 0)
    adts_per_area = distribute(n_enums - n_singletons, n_areas, rng, 0)
    ids_per_area = distribute(n_ids, n_areas, rng, 0)
    wrappers_per_area = distribute(n_wrappers, n_areas, rng, 0)

    endpoints = []
    areas = []
    for ai, area in enumerate(area_names):
        decls = []
        file = f"{area}Models.scala"

        def add(decl):
            decl.area = area
            decl.file = file
            decls.append(decl)
            return u.register(decl)

        for _ in range(ids_per_area[ai]):
            d = Decl(u.names.type_name(words=1, suffix="Id") if rng.chance(0.7) else u.names.type_name(words=2, suffix="Key"), "id", "meridian.model", file)
            d.raw_type = "Long" if rng.chance(0.58) else "String"
            add(d)
        for _ in range(enums_per_area[ai]):
            d = Decl(u.names.type_name(words=rng.pick([1, 2])), "enum", "meridian.model", file)
            d.cases = u.names.case_names(rng.histogram(enum_cases, CASE_BUCKETS))
            d.derives = enum_sets.pop() if enum_sets else ["Enumerated"]
            d.labelled = rng.chance(0.28)
            if rng.chance(0.3):
                d.members.append(f"def isFirst: Boolean = this == {d.name}.{d.cases[0]}")
            if rng.chance(0.5):
                arms = "\n".join(f"    case {d.name}.{c} => \"{rng.pick(vocab.QUALIFIERS)} {c.lower()}\"" for c in d.cases)
                d.members.append(f"def describe: String = this match\n{arms}")
            if rng.chance(0.25) and len(d.cases) > 1:
                d.members.append(f"def isTerminal: Boolean = this match\n    case {d.name}.{d.cases[-1]} => true\n    case _ => false")
            add(d)
        for _ in range(wrappers_per_area[ai]):
            d = Decl(u.names.type_name(words=rng.pick([1, 2])), "wrapper", "meridian.model", file)
            d.raw_type = rng.weighted([("String", 6), ("Long", 2), ("Int", 2)])
            d.givens = ["codec", "schema"] + (["order"] if rng.chance(0.3) else ["eq"] if rng.chance(0.7) else []) + (["show"] if rng.chance(0.4) else []) + (["text"] if rng.chance(0.3) else []) + (["field"] if rng.chance(0.2) else [])
            if rng.chance(0.35):
                d.givens.remove("codec")
                d.givens.append("validated")
                d.check = {"String": "value.nonEmpty", "Long": "value >= 0L", "Int": "value >= 0"}[d.raw_type]
            if rng.chance(0.4):
                d.members.append("def isEmptyValue: Boolean = " + {"String": "value.isEmpty", "Long": "value == 0L", "Int": "value == 0"}[d.raw_type])
            add(d)
        for _ in range(adts_per_area[ai]):
            d = Decl(u.names.type_name(words=rng.pick([1, 2])), "adt", "meridian.model", file)
            d.cases = u.names.case_names(rng.randint(2, 6))
            d.derives = adt_sets.pop() if adt_sets else ["JsonCodec", "Schema"]
            needs = needs_of(d.derives)
            for case in d.cases:
                if rng.chance(0.75):
                    n = rng.randint(1, 3)
                    names = u.names.field_names(n)
                    d.case_fields[case] = [(fname, u.field_type(needs, decls, 1)) for fname in names]
            if rng.chance(0.08):
                d.discriminator = "type"
            if rng.chance(0.3):
                first = d.cases[0]
                d.members.append(f"def isFirstCase: Boolean = this match\n    case _: {d.name}.{first} => true\n    case _ => false" if d.case_fields.get(first) else f"def isFirstCase: Boolean = this == {d.name}.{first}")
            if rng.chance(0.55):
                arms = []
                for c in d.cases:
                    fs = d.case_fields.get(c, [])
                    if fs:
                        arms.append(f"    case {d.name}.{c}({', '.join('_' for _ in fs)}) => \"{c.lower()} of {len(fs)}\"")
                    else:
                        arms.append(f"    case {d.name}.{c} => \"{c.lower()}\"")
                d.members.append("def summary: String = this match\n" + "\n".join(arms))
            add(d)
        entities = []
        for ci in range(per_area[ai]):
            d = Decl(u.names.type_name(words=rng.pick([1, 2, 2])) if ci else area, "case", "meridian.model", file)
            d.derives = class_sets.pop() if class_sets else ["JsonCodec", "Schema"]
            needs = needs_of(d.derives)
            count = rng.histogram(class_fields, FIELD_BUCKETS)
            if "Monoid" in d.derives:
                count = rng.randint(2, 4)
            names = u.names.field_names(count)
            fields = []
            ids = [x for x in decls if x.kind == "id"]
            if ci == 0 and ids and "Monoid" not in d.derives:
                d.role = "entity"
                d.entity_id = ids[0]
                for tc in ("JsonCodec", "Schema"):
                    if tc not in d.derives:
                        d.derives.append(tc)
                d.derives.sort()
                fields.append(("id", u.type_of(ids[0])))
                names = names[:-1] if names else names
            for fname in names:
                if "Monoid" in d.derives:
                    fields.append((fname, rng.pick([PRIMS["Long"], PRIMS["Int"], PRIMS["Amount"]])))
                else:
                    fields.append((fname, u.field_type(needs, decls)))
            d.fields = fields
            if d.role == "entity":
                d.members = make_entity_members(d, rng, [x for x in decls if x.kind == "enum"])
                entities.append(d)
                d.companion.append(f"given Ordering[{d.name}] = Ordering.by(_.id.raw)")
                if rng.chance(0.5):
                    d.companion.append(f"def byId(items: List[{d.name}]): Map[{d.entity_id.name}, {d.name}] = items.map(x => (x.id, x)).toMap")
                if rng.chance(0.4):
                    d.companion.append(f"def newest(items: List[{d.name}]): Option[{d.name}] = items.sorted.lastOption")
            elif rng.chance(0.25) and fields:
                d.members = make_entity_members(d, rng, [x for x in decls if x.kind == "enum"])[:1]
            if rng.chance(0.06) and fields:
                args = [t.sample(rng, 2) for _, t in fields]
                if not any("Samples." in a for a in args):
                    d.companion.append(f"def empty: {d.name} = {d.name}({', '.join(args)})")
            add(d)
        with open(os.path.join(model_dir, file), "w") as f:
            f.write(MODEL_HEADER.format(extra=extra_imports(flags)))
            for d in decls:
                f.write("\n" + render_decl(d, u))
        with open(os.path.join(samples_dir, f"{area}Samples.scala"), "w") as f:
            f.write(SAMPLES_HEADER.format(extra=extra_imports(flags)))
            f.write(f"\nobject {area}Samples:\n")
            for d in decls:
                f.write(f"  val {d.sample_name()}: {d.name} = {sample_value(d, u, rng)}\n")
        areas.append((area, decls, entities))
        endpoints.extend(make_endpoints(u, area, decls, entities, rng))

    routes_files = write_routes(u, api_dir, endpoints, flags, rng)
    write_common(model_dir, api_dir, flags)
    return endpoints, areas, routes_files


def extra_imports(flags):
    lines = []
    if flags.kittens:
        lines.append("import cats.derived.*")
    if flags.cats_ordering:
        lines.append("import cats.kernel.Order.catsKernelOrderingForOrder")
    return "\n".join(lines) + ("\n" if lines else "")


def expand_sets(table, count, rng):
    total = sum(w for _, w in table)
    out = []
    for derives, w in table:
        out += [list(derives) for _ in range(max(0, round(count * w / total)))]
    while len(out) < count:
        out.append(list(table[0][0]))
    rng.shuffle(out)
    return out[:count]


def distribute(total, buckets, rng, minimum):
    counts = [minimum] * buckets
    remaining = total - minimum * buckets
    for _ in range(max(0, remaining)):
        counts[rng.randrange(buckets)] += 1
    return counts


def make_endpoints(u, area, decls, entities, rng):
    out = []
    obj = f"{area}Routes"
    lower = area[0].lower() + area[1:]
    for entity in entities:
        ety = u.type_of(entity)
        idty = u.type_of(entity.entity_id)
        base = [("fixed", "v1"), ("fixed", lower)]
        out.append(EndpointRec(area, obj, f"get{entity.name}", "get", base + [("path", "id")], [("id", idty)], [], [], None, ety, None, "get", entity, "val"))
        filters = [(n, option_of(t)) for n, t in entity.fields if t.text and t.kind in ("prim", "enum", "id") and n not in ("id", "limit")][:2]
        out.append(EndpointRec(area, obj, f"list{entity.name}", "get", base, [], [("limit", option_of(PRIMS["Int"]))] + filters, [], None, list_of(ety), None, "list", entity, "val"))
        if entity.derives and "JsonCodec" in entity.derives:
            out.append(EndpointRec(area, obj, f"create{entity.name}", "post", base, [], [], [], ety, ety, None, "create", entity, "val"))
            out.append(EndpointRec(area, obj, f"update{entity.name}", "put", base + [("path", "id")], [("id", idty)], [], [], ety, None, 204, "update", entity, "val"))
        out.append(EndpointRec(area, obj, f"delete{entity.name}", "delete", base + [("path", "id")], [("id", idty)], [], [], None, None, 204, "delete", entity, "def"))
        for _ in range(rng.randint(0, 3)):
            verb = rng.pick(vocab.VERBS)
            name = verb + entity.name
            if any(e.name == name for e in out):
                continue
            method = rng.weighted([("post", 5), ("get", 4), ("put", 1)])
            segs = base + [("path", "id"), ("fixed", verb)]
            captures = [("id", idty)]
            queries = []
            headers = []
            if rng.chance(0.4):
                queries.append((rng.pick(["since", "until", "page", "mode", "reason", "actor"]), rng.pick([PRIMS["String"], PRIMS["Int"], option_of(PRIMS["LocalDate"]), option_of(PRIMS["String"])])))
            if rng.chance(0.1):
                headers.append(("X-Trace", option_of(PRIMS["String"])))
            body = None
            if method == "post" and rng.chance(0.5):
                candidates = [d for d in decls if d.kind == "case" and d is not entity and "JsonCodec" in d.derives and "Schema" in d.derives]
                if candidates:
                    body = u.type_of(rng.pick(candidates))
            output = ety if rng.chance(0.6) else None
            status = None if output is not None else 204
            out.append(EndpointRec(area, obj, name, method, segs, captures, queries, headers, body, output, status, "custom", entity, rng.pick(["val", "def"])))
    return out


def endpoint_text(e, rng):
    parts = []
    for kind, value in e.segments:
        if kind == "fixed":
            parts.append(f'"{value}"')
        else:
            t = next(t for n, t in e.captures if n == value)
            parts.append(f'path[{t.expr}]("{value}")')
    path = " / ".join(parts)
    text = f"secured.{e.method}.in({path})"
    for name, t in e.queries:
        text += f'.in(query[{t.expr}]("{name}"))'
    for name, t in e.headers:
        text += f'.in(header[{t.expr}]("{name}"))'
    if e.body is not None:
        text += f".in(jsonBody[{e.body.expr}])"
    if e.output is not None:
        text += f".out(jsonOut[{e.output.expr}])"
    if e.status is not None:
        text += f".out(statusCode({e.status}))"
    if rng.chance(0.15):
        text += f'.tag("{e.area.lower()}")'
    return text


def write_routes(u, api_dir, endpoints, flags, rng):
    by_obj = {}
    for e in endpoints:
        by_obj.setdefault(e.obj, []).append(e)
    files = []
    for obj, items in by_obj.items():
        file = f"{obj}.scala"
        with open(os.path.join(api_dir, file), "w") as f:
            f.write(ROUTES_HEADER.format(extra=extra_imports(flags)))
            f.write(f"\nobject {obj}:\n")
            for e in items:
                if e.kind == "def":
                    f.write(f"  def {e.name} = {endpoint_text(e, rng)}\n")
                else:
                    f.write(f"  val {e.name} = {endpoint_text(e, rng)}\n")
            f.write(f"  val all: List[String] = List({', '.join(chr(34) + e.name + chr(34) for e in items)})\n")
        files.append(file)
    return files


def write_common(model_dir, api_dir, flags):
    with open(os.path.join(api_dir, "Security.scala"), "w") as f:
        f.write(ROUTES_HEADER.format(extra=""))
        f.write('''
/** What every private endpoint takes and may answer with. */
enum ApiFailure derives JsonCodec, Schema:
  case NotFound(what: String)
  case Invalid(message: String)
  case Forbidden(reason: String)
  case Conflict(message: String)
  def describe: String = this match
    case NotFound(w) => s"not found: $w"
    case Invalid(m) => s"invalid: $m"
    case Forbidden(r) => s"forbidden: $r"
    case Conflict(m) => s"conflict: $m"

val secured: Endpoint[Credentials, Unit, ApiFailure, Unit] =
  endpoint.securityIn(meridian.core.http.Security.credentials).errorOut(jsonOut[ApiFailure])
''')
    with open(os.path.join(model_dir, "package.scala"), "w") as f:
        f.write('''package meridian.model

import meridian.core.*

/** A page of results, the shape every listing endpoint answers with. */
final case class Page[A](items: List[A], total: Int, next: Option[Int]) derives JsonCodec, Schema:
  def map[B](f: A => B): Page[B] = Page(items.map(f), total, next)
  def isEmpty: Boolean = items.isEmpty

object Page:
  def of[A](items: List[A]): Page[A] = Page(items, items.length, None)
  def empty[A]: Page[A] = Page(Nil, 0, None)
''')
