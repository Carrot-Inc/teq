"""The type universe of the generated application: every model type with its fields, its
derives clause and a sample value, so that the pages, services and self-tests written later refer
to the same types and the same values."""
import random

from . import vocab


class Flags:
    NAMES = ["zio-json", "kittens", "cats-data", "java-time", "big-decimal", "show-interpolator", "cats-ordering", "inline-fc", "conversions", "std-members"]

    def __init__(self, enabled):
        for name in self.NAMES:
            setattr(self, name.replace("-", "_"), name in enabled)
        unknown = [e for e in enabled if e not in self.NAMES]
        if unknown:
            raise SystemExit(f"unknown flags: {', '.join(unknown)}; known: {', '.join(self.NAMES)}")


class Rng(random.Random):
    def pick(self, seq):
        return seq[self.randrange(len(seq))]

    def chance(self, p):
        return self.random() < p

    def weighted(self, pairs):
        total = sum(w for _, w in pairs)
        r = self.random() * total
        for value, w in pairs:
            r -= w
            if r <= 0:
                return value
        return pairs[-1][0]

    def histogram(self, hist, buckets):
        """A value drawn from a survey histogram: bucket label -> count, buckets label -> (lo, hi)."""
        pairs = [(label, count) for label, count in hist.items() if label in buckets]
        label = self.weighted(pairs) if pairs else next(iter(buckets))
        lo, hi = buckets[label]
        return self.randint(lo, hi)


FIELD_BUCKETS = {"0": (0, 0), "1": (1, 1), "2": (2, 2), "3": (3, 3), "4": (4, 4), "5-8": (5, 8), "9-16": (9, 16), "17-32": (17, 32), "33+": (33, 40)}
CASE_BUCKETS = {"1": (1, 1), "2": (2, 2), "3": (3, 3), "4-6": (4, 6), "7-12": (7, 12), "13-24": (13, 24), "25-48": (25, 48), "49+": (49, 60)}
SMALL_BUCKETS = {"0": (0, 0), "1": (1, 1), "2": (2, 2), "3": (3, 3), "4-5": (4, 5), "6-9": (6, 9), "10+": (10, 12)}


class Names:
    """Unique names per scope."""

    def __init__(self, rng):
        self.rng = rng
        self.used = set()

    def type_name(self, words=None, suffix=""):
        for attempt in range(2000):
            n = (words + attempt // 100) if words else self.rng.weighted([(1, 3), (2, 6), (3, 2)])
            parts = [self.rng.pick(vocab.QUALIFIERS) for _ in range(n - 1)] + [self.rng.pick(vocab.NOUNS)]
            name = vocab.camel(parts) + suffix
            if name not in self.used and name not in BLOCKED_TYPES and name.isidentifier():
                self.used.add(name)
                return name
        raise RuntimeError("out of type names")

    def field_names(self, count, extra=()):
        names = set(extra)
        out = []
        while len(out) < count:
            n = self.rng.weighted([(1, 5), (2, 4), (3, 1)])
            parts = [self.rng.pick(vocab.QUALIFIERS) for _ in range(n - 1)] + [self.rng.pick(vocab.NOUNS)]
            if self.rng.chance(0.15):
                parts = [self.rng.pick(vocab.NOUNS), self.rng.pick(vocab.UNITS)]
            name = vocab.lower_camel(parts)
            if name in names or name in RESERVED:
                continue
            names.add(name)
            out.append(name)
        return out

    def case_names(self, count):
        names = []
        pool = list(vocab.CASE_WORDS)
        self.rng.shuffle(pool)
        for w in pool:
            if len(names) == count:
                break
            names.append(w)
        i = 0
        while len(names) < count:
            names.append(vocab.camel([self.rng.pick(vocab.QUALIFIERS), self.rng.pick(vocab.NOUNS)]) + str(i))
            i += 1
        return names

    def value_name(self, base):
        name = base[0].lower() + base[1:]
        i = 2
        candidate = name
        while candidate in self.used:
            candidate = f"{name}{i}"
            i += 1
        self.used.add(candidate)
        return candidate


# Names a generated type must not take: the standard library's, the frameworks', and words that
# read as something else in a type position.
BLOCKED_TYPES = {"Message", "Mailer", "Delivery", "Cache", "MemoryCache", "WorkQueue", "Ack", "Lease", "Profile", "Feature", "Features", "Role", "Permission", "Principal", "Guard", "Policy", "Authenticator", "Sequencer", "Attempt", "Middleware", "Headers", "Trace", "Timing", "Reject", "Fallback", "Counter", "Timer", "Sink", "Registry", "Metrics", "Probe", "Probes", "Health", "Retry", "Access", "Platform", "Criteria", "Listing", "Change", "Entry", "Priority", "Trail", "Budget", "Quota", "Tariff", "Plan", "Slot", "Calendar", "Summary", "Period", "Job", "Outcome", "Scheduled", "Blob", "Document", "Hit", "Index", "Event", "Handler", "Bus", "Clock", "Level", "Line", "Logger", "Fragment", "Param", "Query", "Update", "Table", "Transactor", "Reading", "Station", "Provider", "InstrumentClass", "Planner", "Executor", "Scheduler", "HealthPage", "Blobs", "Tokenizer", "Scoped", "Counting", "Wiring", "Services", "Note", "Grouped", "Ranked", "Column", "Tone", "Meridian", "Allocation", "Console", "Server", "Business", "Infra", "Public", "Core", "Web", "Model", "Api", "Shared", "Admin", "Samples", "Widget", "Component", "Page", "Main", "Router", "Backend", "Apiclient", "Actions", "State", "Routes", "Executor", "View", "Sql", "Events", "Log", "Storage", "Search", "Settings", "Feeds", "Report", "Site", "Map", "List", "Set", "Option", "Vector", "Array", "Seq", "Either", "Some", "None", "Nil", "Unit", "Int", "Long", "String", "Boolean", "Any", "Nothing", "Product", "Iterable", "Ordering", "Numeric", "Tuple", "Function",
                 "Order", "Route", "Routes", "Path", "Layer", "Queue", "Text", "Log", "Ref", "Env", "Tag", "Show", "Eq", "Monoid", "Check", "Fault", "Rules", "Client", "Request", "Response", "Method", "Input", "Output", "Json", "Schema",
                 "Duration", "Instant", "Node", "Router", "Page", "Session", "Credentials", "Transport", "Runtime", "Exit", "Cause", "Task", "Eff", "Lens", "Optional", "Traversal", "Optic", "Iso", "Percent", "Amount", "Several", "Store", "Runner",
                 "Callback", "Renderer", "Hooks", "Reusability", "Rule", "Validator", "Endpoint", "Tw", "Mod", "Enumerated", "Labelled", "LongKey", "TextKey", "Samples", "Main", "Security", "Api", "Checksum", "DayOfWeek", "LocalDate", "LocalTime",
                 "Cluster", "Table", "Chart", "Map", "Marker", "Overlay", "Timeline", "Tab", "Card", "Panel", "Drawer", "Toolbar", "Badge", "Chip", "Toast", "Banner", "Modal", "Wizard", "Stepper", "Widget", "Layout", "Theme", "Gauge", "Sparkline"}
RESERVED = {"type", "val", "var", "def", "class", "object", "match", "case", "if", "else", "then", "do", "yield", "for", "while", "new", "this", "true", "false", "null", "import", "package", "given", "using", "with", "extends", "derives", "enum", "end", "export", "extension", "inline", "opaque", "open", "transparent", "lazy", "sealed", "final", "private", "protected", "override", "abstract", "trait", "return", "throw", "try", "catch", "finally", "super", "macro", "forSome", "implicit", "wait", "notify", "clone", "finalize", "hashCode", "equals", "toString", "getClass", "copy", "apply", "unapply", "productPrefix", "productArity", "canEqual", "ordinal", "values", "valueOf", "label", "raw"}


class Ty:
    """A type usable as a field: its Scala spelling and a sample value in Scala."""

    def __init__(self, expr, sample, kind="prim", key=False, codec=True, eq=True, show=True, order=False, schema=True, text=False):
        self.expr = expr
        self._sample = sample
        self.kind = kind
        self.key = key
        self.codec = codec
        self.eq = eq
        self.show = show
        self.order = order
        self.schema = schema
        self.text = text

    def sample(self, rng, depth=0):
        return self._sample(rng, depth)

    def __repr__(self):
        return f"Ty({self.expr})"


def words_sample(rng, depth=0):
    return '"' + " ".join(rng.pick(vocab.NOUNS) for _ in range(rng.randint(1, 3))) + '"'


PRIMS = {
    "String": Ty("String", words_sample, key=True, order=True, text=True),
    "Int": Ty("Int", lambda r, d: str(r.randint(0, 5000)), key=True, order=True, text=True),
    "Long": Ty("Long", lambda r, d: str(r.randint(0, 900000)) + "L", key=True, order=True, text=True),
    "Boolean": Ty("Boolean", lambda r, d: r.pick(["true", "false"]), order=False, text=True),
    "Instant": Ty("Instant", lambda r, d: f"Instant.ofEpochSecond({r.randint(1600000000, 1800000000)}L)", order=True, text=True),
    "LocalDate": Ty("LocalDate", lambda r, d: f"LocalDate.of({r.randint(2020, 2027)}, {r.randint(1, 12)}, {r.randint(1, 28)})", key=True, order=True, text=True),
    "LocalTime": Ty("LocalTime", lambda r, d: f"LocalTime.of({r.randint(0, 23)}, {r.pick(['0', '15', '30', '45'])})", order=True),
    "Duration": Ty("Duration", lambda r, d: f"Duration.ofMinutes({r.randint(1, 600)}L)", show=True),
    "Amount": Ty("Amount", lambda r, d: f'Amount.unsafeParse("{r.randint(0, 900)}.{r.randint(0, 99):02d}")', order=True),
    "PosInt": Ty("PosInt", lambda r, d: f"PosInt.unsafeFrom({r.randint(1, 500)})", show=False),
    "NonNegInt": Ty("NonNegInt", lambda r, d: f"NonNegInt.unsafeFrom({r.randint(0, 500)})", show=False),
    "Percent": Ty("Percent", lambda r, d: f"Percent.unsafeFrom({r.randint(0, 100)})", show=False),
    "NonEmptyText": Ty("NonEmptyText", lambda r, d: f'NonEmptyText.unsafeFrom("{r.pick(vocab.NOUNS)}")', show=False),
}

PRIM_WEIGHTS = [("String", 30), ("Int", 10), ("Long", 8), ("Boolean", 12), ("Instant", 9), ("LocalDate", 5), ("LocalTime", 2), ("Duration", 1), ("Amount", 6), ("PosInt", 1), ("NonNegInt", 1), ("Percent", 1), ("NonEmptyText", 1)]


def option_of(t):
    return Ty(f"Option[{t.expr}]", lambda r, d: f"Some({t.sample(r, d + 1)})" if r.chance(0.7) else "None", kind="option", codec=t.codec, eq=t.eq, show=t.show, order=t.order, schema=t.schema, text=t.text)


def list_of(t):
    return Ty(f"List[{t.expr}]", lambda r, d: "List(" + ", ".join(t.sample(r, d + 1) for _ in range(r.randint(0 if d > 1 else 1, 3 if d < 2 else 1))) + ")", kind="list", codec=t.codec, eq=t.eq, show=t.show, order=t.order, schema=t.schema, text=t.text)


def set_of(t):
    return Ty(f"Set[{t.expr}]", lambda r, d: "Set(" + ", ".join(t.sample(r, d + 1) for _ in range(r.randint(1, 2))) + ")", kind="set", codec=t.codec, eq=t.eq, show=t.show, schema=t.schema)


def several_of(t):
    return Ty(f"Several[{t.expr}]", lambda r, d: "Several.of(" + ", ".join(t.sample(r, d + 1) for _ in range(r.randint(1, 2))) + ")", kind="several", codec=t.codec, eq=t.eq, show=t.show, schema=t.schema)


def map_of(k, v):
    return Ty(f"Map[{k.expr}, {v.expr}]", lambda r, d: "Map(" + ", ".join(f"{k.sample(r, d + 1)} -> {v.sample(r, d + 1)}" for _ in range(r.randint(1, 2))) + ")", kind="map", codec=k.key and v.codec, eq=v.eq, show=v.show and k.show, schema=v.schema)


class Decl:
    """A generated type: a case class, an enum (singleton cases), an adt (class cases), an id or a wrapper."""

    def __init__(self, name, kind, package, file):
        self.name = name
        self.kind = kind
        self.package = package
        self.file = file
        self.fields = []
        self.cases = []
        self.case_fields = {}
        self.derives = []
        self.labelled = False
        self.discriminator = None
        self.members = []
        self.companion = []
        self.role = "value"
        self.raw_type = "Long"
        self.entity_id = None
        self.ty = None
        self.givens = []
        self.check = "true"
        self.area = ""

    def has(self, tc):
        return tc in self.derives

    def sample_name(self):
        return self.name[0].lower() + self.name[1:]


class Universe:
    def __init__(self, rng, flags):
        self.rng = rng
        self.flags = flags
        self.names = Names(rng)
        self.decls = []
        self.by_module = {}
        self.enums = []
        self.adts = []
        self.classes = []
        self.ids = []
        self.wrappers = []
        self.entities = []

    def register(self, decl):
        self.decls.append(decl)
        self.by_module.setdefault(decl.package, []).append(decl)
        {"case": self.classes, "enum": self.enums, "adt": self.adts, "id": self.ids, "wrapper": self.wrappers}[decl.kind].append(decl)
        return decl

    def sample_ref(self, decl, depth=0):
        return f"{decl.area}Samples.{decl.sample_name()}"

    def type_of(self, decl):
        if decl.ty is None:
            if decl.kind == "id":
                if decl.raw_type == "Long":
                    decl.ty = Ty(decl.name, lambda r, d, n=decl.name: f"{n}({r.randint(1, 9000)}L)", kind="id", key=True, order=True, text=True)
                else:
                    decl.ty = Ty(decl.name, lambda r, d, n=decl.name: f'{n}("{r.pick(vocab.NOUNS)}-{r.randint(1, 99)}")', kind="id", key=True, order=True, text=True)
            elif decl.kind == "wrapper":
                inner = PRIMS[decl.raw_type]
                g = decl.givens
                decl.ty = Ty(decl.name, lambda r, d, n=decl.name, i=inner: f"{n}({i.sample(r, d)})", kind="wrapper", key="field" in g,
                             codec="codec" in g or "validated" in g, eq="eq" in g or "order" in g, show="show" in g, order="order" in g, schema="schema" in g, text="text" in g)
            elif decl.kind == "enum":
                decl.ty = Ty(decl.name, lambda r, d, n=decl.name, cs=decl.cases: f"{n}.{r.pick(cs)}", kind="enum", key=decl.has("Enumerated"), codec=decl.has("JsonCodec") or decl.has("Enumerated"), eq=decl.has("Eq") or decl.has("Enumerated"), show=decl.has("Show") or decl.has("Enumerated"), order=decl.has("Order") or decl.has("Enumerated"), schema=decl.has("Schema") or decl.has("Enumerated"), text=decl.has("Enumerated"))
            elif decl.kind == "adt":
                decl.ty = Ty(decl.name, lambda r, d, dc=decl: self.sample_ref(dc, d), kind="adt", codec=decl.has("JsonCodec"), eq=decl.has("Eq"), show=decl.has("Show") or decl.labelled, schema=decl.has("Schema"))
            else:
                decl.ty = Ty(decl.name, lambda r, d, dc=decl: self.sample_ref(dc, d), kind="case", codec=decl.has("JsonCodec"), eq=decl.has("Eq"), show=decl.has("Show"), schema=decl.has("Schema"), order=decl.has("Order"))
        return decl.ty

    def field_type(self, needs, pool, depth=0, models_allowed=True):
        """A field type whose instances satisfy `needs` (a subset of codec/eq/show/schema/order),
        with the model, enum and id types drawn from `pool`, the declarations already made."""
        rng = self.rng
        ids = [d for d in pool if d.kind == "id"]
        enums = [d for d in pool if d.kind == "enum"]
        models = [d for d in pool if d.kind in ("case", "adt", "wrapper")]
        wrappers = 0 if depth >= 2 else 1
        for _ in range(50):
            choice = rng.weighted([("prim", 52), ("option", 18 * wrappers), ("list", 9 * wrappers), ("id", 8), ("enum", 7), ("model", 6 if models_allowed and depth < 2 else 0), ("map", 2 * wrappers), ("set", 1 * wrappers), ("several", 1 * wrappers)])
            if choice == "prim":
                t = PRIMS[rng.weighted(PRIM_WEIGHTS)]
            elif choice == "id" and ids:
                t = self.type_of(rng.pick(ids))
            elif choice == "enum" and enums:
                t = self.type_of(rng.pick(enums))
            elif choice == "model" and models:
                t = self.type_of(rng.pick(models))
            elif choice == "option":
                inner = self.field_type(needs, pool, depth + 1, models_allowed)
                if inner.kind == "option":
                    continue
                t = option_of(inner)
            elif choice == "list":
                t = list_of(self.field_type(needs, pool, depth + 1, models_allowed))
            elif choice == "set":
                t = set_of(self.field_type(needs, pool, depth + 1, False))
            elif choice == "several":
                t = several_of(self.field_type(needs, pool, depth + 1, models_allowed))
            elif choice == "map":
                keys = [PRIMS["String"], PRIMS["LocalDate"]] + [self.type_of(d) for d in ids[-3:]] + [self.type_of(d) for d in enums if d.has("Enumerated")][-3:]
                t = map_of(rng.pick(keys), self.field_type(needs, pool, depth + 1, models_allowed))
            else:
                continue
            if all(getattr(t, n) for n in needs):
                return t
        return PRIMS["String"]


def needs_of(derives):
    needs = []
    if "JsonCodec" in derives:
        needs.append("codec")
    if "Schema" in derives:
        needs.append("schema")
    if "Eq" in derives:
        needs.append("eq")
    if "Show" in derives:
        needs.append("show")
    if "Order" in derives:
        needs.append("order")
    return needs
