"""The API side: per area a repository over an in-memory table addressed through SQL fragments,
a service over the effect type, validation and mapping rules, reports; the server's routes and
wiring, an executor, the infrastructure (sql, events, logging, storage, search, settings and the
provider clients of the feeds), the allocation package, the public endpoint definitions, and a
self-test that sends a request to every endpoint and prints a checksum."""
import os

from . import vocab
from .frontend_area import Call
from .emit import sample_value
from .shared import scaled

HEADER = """package {pkg}

import meridian.core.*
import meridian.core.Codecs.given
import meridian.core.http.{{Credentials, Request, Response}}
import meridian.model.*
import meridian.model.api.*
import meridian.infra.sql.*
import meridian.infra.events.*
import meridian.infra.log.*
import cats.syntax.all.*
{extra}
"""


def lower(name):
    return name[0].lower() + name[1:]


class Ctx:
    def __init__(self, u, endpoints, areas, out, rng, flags, scale):
        self.u = u
        self.endpoints = endpoints
        self.areas = areas
        self.out = os.path.join(out, "api")
        self.rng = rng
        self.flags = flags
        self.scale = scale
        self.files = 0
        self.by_area = {area: [e for e in endpoints if e.area == area] for area, _, _ in areas}
        self.providers = []

    def write(self, pkg, name, body, extra=""):
        parts = pkg.split(".")
        d = os.path.join(self.out, *parts)
        os.makedirs(d, exist_ok=True)
        with open(os.path.join(d, name + ".scala"), "w") as f:
            f.write(HEADER.format(pkg="meridian." + pkg, extra=extra) + "\n" + body)
        self.files += 1


def write_api(u, endpoints, areas, out, targets, rng, flags, scale):
    ctx = Ctx(u, endpoints, areas, out, rng, flags, scale)
    write_infra(ctx)
    for area, decls, entities in areas:
        write_business_area(ctx, area, decls, entities)
    write_allocation(ctx)
    write_public(ctx)
    write_server(ctx)
    return {"files": ctx.files}


# ---------------------------------------------------------------------------------------------
# infra

ELEM_GIVENS = {
    "conversions": "  given fromPut[A](using put: Put[A]): Conversion[A, Elem] = a => Arg(put.param(a))",
    "monomorphic": """  given fromText: Conversion[String, Elem] = s => Arg(Param.Text(s))
  given fromInt: Conversion[Int, Elem] = i => Arg(Param.Int(i))
  given fromLong: Conversion[Long, Elem] = l => Arg(Param.Long(l))
  given fromBool: Conversion[Boolean, Elem] = b => Arg(Param.Bool(b))""",
}


def bound(ctx, expr):
    """A key or enum value interpolated into a fragment: bare with the Put conversion, else bound by hand."""
    return "${" + expr + "}" if ctx.flags.conversions else "${" + expr + ".bind}"


def write_infra(ctx):
    ctx.write("infra.sql", "Fragment", '''/** A piece of SQL with its parameters, built by the `sql` interpolator and composed with `++`. */
final case class Fragment(text: String, params: List[Param]):
  def ++(that: Fragment): Fragment = Fragment(text + " " + that.text, params ++ that.params)
  def where(condition: Fragment): Fragment = this ++ Fragment("where", Nil) ++ condition
  def and(condition: Fragment): Fragment = this ++ Fragment("and", Nil) ++ condition
  def orderBy(column: String): Fragment = this ++ Fragment(s"order by $column", Nil)
  def limit(n: Int): Fragment = this ++ Fragment("limit ?", List(Param.Int(n)))
  def render: String = text.replaceAll("\\\\s+", " ").trim
  def query[A](using read: Read[A]): Query[A] = Query(this, read)
  def update: Update = Update(this)

object Fragment:
  val empty: Fragment = Fragment("", Nil)
  def const(text: String): Fragment = Fragment(text, Nil)
  def in[A](values: List[A])(using put: Put[A]): Fragment =
    if values.isEmpty then Fragment("(null)", Nil)
    else Fragment(values.map(_ => "?").mkString("(", ", ", ")"), values.map(put.param))
  def and(conditions: List[Fragment]): Fragment = conditions match
    case Nil => Fragment("true", Nil)
    case head :: tail => tail.foldLeft(head)((acc, c) => acc ++ Fragment("and", Nil) ++ c)

enum Param:
  case Text(value: String)
  case Int(value: scala.Int)
  case Long(value: scala.Long)
  case Bool(value: Boolean)
  def render: String = this match
    case Text(v) => "'" + v + "'"
    case Int(v) => v.toString
    case Long(v) => v.toString
    case Bool(v) => v.toString

/** A value written into a fragment. */
trait Put[A]:
  def param(a: A): Param
  def contramap[B](f: B => A): Put[B] = b => param(f(b))
object Put:
  given Put[String] = Param.Text(_)
  given Put[Int] = Param.Int(_)
  given Put[Long] = Param.Long(_)
  given Put[Boolean] = Param.Bool(_)
  given [K](using key: LongKey[K]): Put[K] = k => Param.Long(k.raw)
  given [K](using key: TextKey[K]): Put[K] = k => Param.Text(k.raw)
  given [E](using e: Enumerated[E]): Put[E] = v => Param.Text(v.entryName)
  given Put[Instant] = i => Param.Long(i.toEpochMilli)
  given Put[LocalDate] = d => Param.Text(d.toString)
  given [A](using p: Put[A]): Put[Option[A]] = o => o.map(p.param).getOrElse(Param.Text("null"))

/** A row read back: the in-memory transactor reads nothing, it hands rows through. */
trait Read[A]:
  def name: String
object Read:
  def named[A](n: String): Read[A] = new Read[A]:
    def name = n
  given any[A]: Read[A] = named("row")

final case class Query[A](fragment: Fragment, read: Read[A])
final case class Update(fragment: Fragment)

/** An interpolated argument: a fragment spliced as text, or a value with a Put written as a parameter. */
enum Elem:
  case Splice(fragment: Fragment)
  case Arg(param: Param)
object Elem:
  given fromFragment: Conversion[Fragment, Elem] = Splice(_)
  given fromParam: Conversion[Param, Elem] = Arg(_)
__ELEM_GIVENS__

extension [A](a: A) def bind(using put: Put[A]): Param = put.param(a)

extension (sc: StringContext)
  def sql(args: Elem*): Fragment =
    val parts = sc.parts.toList
    val (text, params) = parts.zip(args.toList :+ Elem.Splice(Fragment.empty)).foldLeft(("", List.empty[Param])) { case ((t, ps), (part, arg)) =>
      arg match
        case Elem.Splice(f) => (t + part + f.text, ps ++ f.params)
        case Elem.Arg(p) => (t + part + "?", ps :+ p)
    }
    Fragment(text, params)
  def fr(args: Elem*): Fragment = sql(args*)
'''.replace("__ELEM_GIVENS__", ELEM_GIVENS["conversions" if ctx.flags.conversions else "monomorphic"]), extra="")
    ctx.write("infra.sql", "Transactor", '''/** Runs queries against in-memory tables, keeping the SQL text of every statement it saw. */
final class Table[K, V](initial: List[V], key: V => K):
  private val ref: Ref[Map[K, V]] = Ref.unsafeMake(initial.map(v => (key(v), v)).toMap)
  def all: UIO[List[V]] = ref.get.map(_.values.toList)
  def get(k: K): UIO[Option[V]] = ref.get.map(_.get(k))
  def put(v: V): UIO[Unit] = ref.update(_.updated(key(v), v))
  def remove(k: K): UIO[Boolean] = ref.modify(m => (m.contains(k), m.removed(k)))
  def size: UIO[Int] = ref.get.map(_.size)
  def where(p: V => Boolean): UIO[List[V]] = all.map(_.filter(p))

final class Transactor:
  private var statements: List[String] = Nil
  private var counts: Map[String, Int] = Map.empty
  def run[A](query: Query[A])(rows: UIO[List[A]]): Task[List[A]] =
    record(query.fragment) *> rows
  def one[A](query: Query[A])(row: UIO[Option[A]]): Task[Option[A]] =
    record(query.fragment) *> row
  def execute(update: Update)(effect: UIO[Int]): Task[Int] =
    record(update.fragment) *> effect
  def transactionally[A](steps: Task[A]): Task[A] =
    Eff.succeed { statements = "begin" :: statements } *> steps <* Eff.succeed { statements = "commit" :: statements }
  private def record(fragment: Fragment): UIO[Unit] = Eff.succeed {
    val text = fragment.render
    statements = text :: statements
    val verb = text.takeWhile(_ != ' ')
    counts = counts.updated(verb, counts.getOrElse(verb, 0) + 1)
  }
  def seen: List[String] = statements.reverse
  def summary: String = counts.toList.sortBy(_._1).map((k, v) => s"$k=$v").mkString(", ")

object Transactor:
  def make: Transactor = new Transactor
''', extra="")
    ctx.write("infra.events", "Events", '''/** Domain events and the bus that hands them to their handlers. */
enum Event:
  case Created(kind: String, id: String, at: Instant)
  case Updated(kind: String, id: String, fields: List[String], at: Instant)
  case Removed(kind: String, id: String, at: Instant)
  case Custom(kind: String, id: String, action: String, at: Instant)
  case Failed(kind: String, reason: String, at: Instant)
  def kind: String
  def describe: String = this match
    case Created(k, id, _) => s"created $k $id"
    case Updated(k, id, fields, _) => s"updated $k $id (${fields.mkString(",")})"
    case Removed(k, id, _) => s"removed $k $id"
    case Custom(k, id, action, _) => s"$action $k $id"
    case Failed(k, reason, _) => s"failed $k: $reason"

trait Handler:
  def name: String
  def accepts(event: Event): Boolean
  def handle(event: Event): UIO[Unit]

final class Bus:
  private var handlers: List[Handler] = Nil
  private var log: List[Event] = Nil
  def subscribe(handler: Handler): UIO[Unit] = Eff.succeed { handlers = handlers :+ handler }
  def publish(event: Event): UIO[Unit] =
    Eff.succeed { log = event :: log } *> Eff.foreachDiscard(handlers.filter(_.accepts(event)))(_.handle(event))
  def history: List[Event] = log.reverse
  def count: Int = log.length
  def kinds: Map[String, Int] = log.groupBy(_.kind).map((k, v) => (k, v.length))

object Bus:
  def make: Bus = new Bus

final class Counting(val name: String, kinds: Set[String]) extends Handler:
  private var seen: Int = 0
  def accepts(event: Event) = kinds.isEmpty || kinds.contains(event.kind)
  def handle(event: Event) = Eff.succeed { seen += 1 }
  def total: Int = seen

trait Clock:
  def now: Instant
  def today: LocalDate
object Clock:
  def fixed(at: Instant): Clock = new Clock:
    def now = at
    def today = at.toLocalDate
  def ticking(start: Instant, step: Duration): Clock = new Clock:
    private var current = start
    def now =
      current = current.plus(step)
      current
    def today = current.toLocalDate
''', extra="")
    ctx.write("infra.log", "Logger", '''/** Structured lines kept in memory, printed by the self-test. */
enum Level derives Enumerated, Eq, Order:
  case Debug, Info, Warn, Error

final case class Line(level: Level, scope: String, text: String, fields: Map[String, String] = Map.empty):
  def render: String =
    val extra = if fields.isEmpty then "" else fields.toList.sortBy(_._1).map((k, v) => s"$k=$v").mkString(" ", " ", "")
    s"${level.entryName} [$scope] $text$extra"

final class Logger(minimum: Level):
  private var lines: List[Line] = Nil
  def log(line: Line): UIO[Unit] = Eff.succeed { if Enumerated[Level].ordinalOf(line.level) >= Enumerated[Level].ordinalOf(minimum) then lines = line :: lines }
  def debug(scope: String, text: String): UIO[Unit] = log(Line(Level.Debug, scope, text))
  def info(scope: String, text: String, fields: (String, String)*): UIO[Unit] = log(Line(Level.Info, scope, text, fields.toMap))
  def warn(scope: String, text: String): UIO[Unit] = log(Line(Level.Warn, scope, text))
  def error(scope: String, text: String): UIO[Unit] = log(Line(Level.Error, scope, text))
  def drain: List[Line] =
    val out = lines.reverse
    lines = Nil
    out
  def count: Int = lines.length
  def countBy(level: Level): Int = lines.count(_.level === level)

object Logger:
  def make(minimum: Level = Level.Info): Logger = new Logger(minimum)

def scoped(scope: String): Scoped = Scoped(scope)
final case class Scoped(scope: String):
  def info(text: String, fields: (String, String)*): Eff[Logger, Nothing, Unit] = Eff.service[Logger].flatMap(_.info(scope, text, fields*))
  def warn(text: String): Eff[Logger, Nothing, Unit] = Eff.service[Logger].flatMap(_.warn(scope, text))
  def error(text: String): Eff[Logger, Nothing, Unit] = Eff.service[Logger].flatMap(_.error(scope, text))
  def debug(text: String): Eff[Logger, Nothing, Unit] = Eff.service[Logger].flatMap(_.debug(scope, text))
''', extra="")
    ctx.write("infra.storage", "Blobs", '''/** Named blobs with a content type, in memory. */
final case class Blob(name: String, contentType: String, bytes: List[Int], createdAt: Instant) derives JsonCodec:
  def size: Int = bytes.length
  def checksum: Long = bytes.foldLeft(7L)((h, b) => h * 31L + b)

enum BlobError derives Eq:
  case NotFound(name: String)
  case TooLarge(name: String, size: Int)
  case BadName(name: String)

final class Blobs(limit: Int):
  private val ref: Ref[Map[String, Blob]] = Ref.unsafeMake(Map.empty)
  def put(blob: Blob): IO[BlobError, Blob] =
    if blob.name.isEmpty then Eff.fail(BlobError.BadName(blob.name))
    else if blob.size > limit then Eff.fail(BlobError.TooLarge(blob.name, blob.size))
    else ref.update(_.updated(blob.name, blob)).as(blob)
  def get(name: String): IO[BlobError, Blob] = ref.get.flatMap(m => Eff.fromOption(m.get(name)).mapError(_ => BlobError.NotFound(name)))
  def list: UIO[List[Blob]] = ref.get.map(_.values.toList.sortBy(_.name))
  def remove(name: String): UIO[Boolean] = ref.modify(m => (m.contains(name), m.removed(name)))
  def totalSize: UIO[Int] = list.map(_.map(_.size).sum)

object Blobs:
  def make(limit: Int = 1 << 16): Blobs = new Blobs(limit)
  def text(name: String, text: String, at: Instant): Blob = Blob(name, "text/plain", ''' + ("text.map(_.toInt).toList" if ctx.flags.std_members else "text.toList.map(_.toInt)") + ''', at)
''', extra="")
    ctx.write("infra.search", "Index", '''/** A word index over documents, the shape of the search integration. */
final case class Document(id: String, kind: String, fields: Map[String, String], boost: Int = 1)
final case class Hit(id: String, kind: String, score: Int)

object Tokenizer:
  def tokens(text: String): List[String] =
    text.toLowerCase.split("[^a-z0-9]+").toList.filter(t => t.length > 1)
  def stem(token: String): String = token match
    case t if t.endsWith("ies") => t.dropRight(3) + "y"
    case t if t.endsWith("es") => t.dropRight(2)
    case t if t.endsWith("s") && t.length > 3 => t.dropRight(1)
    case t => t

final class Index:
  private var docs: Map[String, Document] = Map.empty
  private var words: Map[String, Set[String]] = Map.empty
  def add(doc: Document): UIO[Unit] = Eff.succeed {
    docs = docs.updated(doc.id, doc)
    for (_, text) <- doc.fields; token <- Tokenizer.tokens(text).map(Tokenizer.stem) do
      words = words.updated(token, words.getOrElse(token, Set.empty) + doc.id)
  }
  def remove(id: String): UIO[Unit] = Eff.succeed {
    docs = docs.removed(id)
    words = words.map((w, ids) => (w, ids - id))
  }
  def search(query: String, kind: Option[String] = None, limit: Int = 10): UIO[List[Hit]] = Eff.succeed {
    val terms = Tokenizer.tokens(query).map(Tokenizer.stem)
    val scored = docs.values.toList.filter(d => kind.forall(_ == d.kind)).map { d =>
      val score = terms.count(t => words.getOrElse(t, Set.empty).contains(d.id)) * d.boost
      Hit(d.id, d.kind, score)
    }
    scored.filter(_.score > 0).sortBy(h => (-h.score, h.id)).take(limit)
  }
  def size: Int = docs.size
  def vocabulary: Int = words.size

object Index:
  def make: Index = new Index
''', extra="")
    write_settings(ctx)
    write_feeds(ctx)


def write_settings(ctx):
    rng = ctx.rng
    for i in range(8):
        name = vocab.camel([rng.pick(vocab.QUALIFIERS), rng.pick(vocab.NOUNS)]) + "Setting"
        while name in ctx.u.names.used:
            name = vocab.camel([rng.pick(vocab.QUALIFIERS), rng.pick(vocab.NOUNS)]) + "Setting"
        ctx.u.names.used.add(name)
        cases = ctx.u.names.case_names(rng.randint(4, 12))
        lines = [f"/** A setting of the site, its keys enumerated and each key typed. */", f"enum {name}(val label: String, val default: String) derives Enumerated, Eq:"]
        for c in cases:
            lines.append(f'  case {c} extends {name}("{c.lower()}", "{rng.pick(vocab.NOUNS)}")')
        lines += [f"  def key: String = \"{name.lower()}.\" + label", "", f"object {name}:",
                  f"  def parse(text: String): Option[{name}] = Enumerated[{name}].valueOfIgnoreCase(text)",
                  f"  def defaults: Map[{name}, String] = Enumerated[{name}].valueList.map(s => (s, s.default)).toMap",
                  f"  final case class Values(values: Map[{name}, String]) derives JsonCodec, Schema:",
                  f"    def get(setting: {name}): String = values.getOrElse(setting, setting.default)",
                  f"    def set(setting: {name}, value: String): Values = Values(values.updated(setting, value))",
                  f"    def changed: List[{name}] = values.toList.filter((k, v) => v != k.default).map(_._1)",
                  f"  def describe(setting: {name}): String = setting match"]
        for c in cases[:6]:
            lines.append(f"    case {name}.{c} => \"{rng.pick(vocab.QUALIFIERS)} {c.lower()}\"")
        if len(cases) > 6:
            lines.append("    case other => other.label")
        ctx.write("infra.settings", name, "\n".join(lines) + "\n")


def write_feeds(ctx):
    rng = ctx.rng
    n = max(2, scaled(16, ctx.scale))
    names = []
    while len(names) < n:
        p = vocab.camel([rng.pick(vocab.PLACE_NAMES)]) + rng.pick(["Feed", "Link", "Relay", "Bridge"])
        if p not in names and p not in ctx.u.names.used:
            names.append(p)
            ctx.u.names.used.add(p)
    ctx.providers = names
    ctx.write("infra.feeds", "Provider", '''/** An external source of readings: every provider speaks its own wire types and is mapped
  * onto these. */
final case class Reading(source: String, station: String, at: Instant, value: Long, unit: String, flags: List[String]) derives JsonCodec, Eq, Show
final case class Station(source: String, code: String, name: String, active: Boolean) derives JsonCodec, Eq

enum ProviderError derives Eq, Show:
  case Unreachable(source: String)
  case Malformed(source: String, detail: String)
  case Unauthorised(source: String)
  case RateLimited(source: String, retryAfter: Duration)

trait Provider:
  def name: String
  def stations: IO[ProviderError, List[Station]]
  def readings(station: String, since: Instant): IO[ProviderError, List[Reading]]
  def health: UIO[Boolean]

final case class ProviderReport(name: String, stations: Int, readings: Int, healthy: Boolean, errors: List[String]):
  def render: String = s"$name: $stations stations, $readings readings, ${if healthy then "healthy" else "unhealthy"}${if errors.isEmpty then "" else errors.mkString(" (", "; ", ")")}"

object Providers:
  def report(provider: Provider, since: Instant): UIO[ProviderReport] =
    for
      healthy <- provider.health
      stations <- provider.stations.either
      readings <- stations match
        case Right(list) => Eff.foreach(list.take(3))(s => provider.readings(s.code, since).either).map(_.collect { case Right(rs) => rs.length }.sum)
        case Left(_) => Eff.pure(0)
      errors = stations.left.toOption.map(_.toString).toList
    yield ProviderReport(provider.name, stations.map(_.length).getOrElse(0), readings, healthy, errors)
''')
    for p in names:
        write_provider(ctx, p)
    write_infra_extras(ctx)


def wire_derives(wanted, nested):
    """The derives clause a wire type can have: no more than the type it nests provides."""
    if nested is not None:
        parts = {"JsonEncoder", "JsonDecoder"} if "JsonCodec" in nested else nested & {"JsonEncoder", "JsonDecoder"}
        wanted_parts = {"JsonEncoder", "JsonDecoder"} if "JsonCodec" in wanted else wanted & {"JsonEncoder", "JsonDecoder"}
        allowed = (wanted_parts & parts) or parts
        wanted = allowed | ({"Eq"} if "Eq" in wanted and "Eq" in nested else set())
    if {"JsonEncoder", "JsonDecoder"} <= wanted or "JsonCodec" in wanted:
        return "JsonCodec, Eq" if "Eq" in wanted else "JsonCodec"
    codec = "JsonEncoder" if "JsonEncoder" in wanted else "JsonDecoder"
    return codec + (", Eq" if "Eq" in wanted else "")


MEASURES = ["seconds", "minutes", "hours", "days", "millimetres", "metres", "degrees", "arcsec", "nanometres", "kelvin", "percent"]


def raw_json(ctx, pkg, model_name, rng):
    """Two rows of the wire type as its API would send them, one field per line."""
    fields = wire_fields(ctx, pkg, model_name)
    rows = []
    for row in range(2):
        parts = []
        for name, tpe in fields:
            if tpe == "String":
                value = f'"{rng.pick(vocab.NOUNS)}"'
            elif tpe == "Int":
                value = str(rng.randint(0, 900))
            elif tpe == "Long":
                value = str(rng.randint(0, 90000))
            elif tpe == "Boolean":
                value = rng.pick(["true", "false"])
            elif tpe == "Option[String]":
                value = f'"{rng.pick(vocab.QUALIFIERS)}"'
            elif tpe in ("Option[Int]", "Option[Long]"):
                value = str(rng.randint(1, 99))
            elif tpe == "List[String]":
                value = f'["{rng.pick(vocab.NOUNS)}", "{rng.pick(vocab.NOUNS)}"]'
            else:
                value = "[]"
            parts.append(f'      "{name}": {value}')
        rows.append("    {\n" + ",\n".join(parts) + "\n    }")
    return ",\n".join(rows)


def wire_fields(ctx, pkg, model_name):
    text = ""
    for dirpath, _, names in os.walk(os.path.join(ctx.out, *pkg.split("."))):
        for n in names:
            if n.endswith("Models.scala"):
                text = open(os.path.join(dirpath, n)).read()
    start = text.index(f"final case class {model_name}(") + len(f"final case class {model_name}(")
    end = text.index("\n)", start)
    fields = []
    for line in text[start:end].split("\n"):
        line = line.strip().rstrip(",")
        if line:
            name, tpe = line.split(": ", 1)
            fields.append((name, tpe))
    return fields


SYNC = """/** Pulling @P@ into the readings the services use, with what the last run saw. */
object @P@Sync:
  final case class SyncState(
    lastRun: Option[Instant] = None,
    seen: Int = 0,
    failures: Int = 0,
    stations: List[String] = Nil
  )

  final case class SyncReport(provider: String, stations: Int, readings: Int, skipped: Int, failures: List[String], took: Duration):
    def render: String =
      provider + ": " + stations + " stations, " + readings + " readings, " + skipped + " skipped" + (if failures.isEmpty then "" else ", failed " + failures.mkString("/"))

  def run(client: @P@Client, clock: Clock, state: Ref[SyncState], since: Instant): UIO[SyncReport] =
    for
      started <- Eff.succeed(clock.now)
      current <- state.get
      stations <- client.stations.either
      active = stations.fold(_ => Nil, _.filter(_.active))
      results <- Eff.foreach(active.take(4))(s => client.readings(s.code, since).either.map(r => (s, r)))
      readings = results.collect { case (_, Right(rs)) => rs }.flatten
      failed = results.collect { case (s, Left(e)) => s.code + ":" + e.toString }
      skipped = active.length - results.length
      _ <- state.set(SyncState(lastRun = Some(clock.now), seen = current.seen + readings.length, failures = current.failures + failed.length, stations = active.map(_.code)))
    yield SyncReport(client.name, active.length, readings.length, skipped, failed ++ stations.left.toOption.map(_.toString).toList, Duration.between(started, clock.now))

  def due(state: SyncState, now: Instant, every: Duration): Boolean = state.lastRun match
    case None => true
    case Some(last) => !last.plus(every).isAfter(now)

  def merge(reports: List[SyncReport]): SyncReport =
    reports.foldLeft(SyncReport("@P@", 0, 0, 0, Nil, Duration.ofMillis(0L))) { (acc, r) =>
      SyncReport(acc.provider, acc.stations + r.stations, acc.readings + r.readings, acc.skipped + r.skipped, acc.failures ++ r.failures, acc.took.plus(r.took))
    }

  def verdict(report: SyncReport): String = (report.readings, report.failures.length) match
    case (0, 0) => "nothing to do"
    case (0, _) => "all failed"
    case (_, 0) => "clean"
    case (n, f) if f > n => "mostly failed"
    case _ => "partial"

  def latestBy(readings: List[Reading]): Map[String, Reading] =
    readings.groupBy(_.station).flatMap((station, rs) => @P@Mapping.latest(rs).map(r => (station, r)))
"""

CACHE = """/** The @P@ source behind a memo: answers repeated within the ttl come from the last fetch. */
trait @P@Source:
  def stations: IO[ProviderError, List[Station]]
  def readings(station: String, since: Instant): IO[ProviderError, List[Reading]]

final class @P@Cache(source: @P@Source, ttl: Duration, clock: Clock) extends @P@Source:
  private val stationsMemo: Ref[Option[(Instant, List[Station])]] = Ref.unsafeMake(None)
  private val readingsMemo: Ref[Map[String, (Instant, List[Reading])]] = Ref.unsafeMake(Map.empty)
  private var hits: Int = 0
  private var misses: Int = 0

  private def fresh[A](entry: Option[(Instant, A)]): Option[A] =
    entry.filter((at, _) => !at.plus(ttl).isBefore(clock.now)).map(_._2)

  def stations: IO[ProviderError, List[Station]] =
    for
      cached <- stationsMemo.get
      result <- fresh(cached).map(items => Eff.succeed { hits += 1 }.as(items)).getOrElse(
        source.stations.flatMap(items => stationsMemo.set(Some((clock.now, items))).map { _ => misses += 1; items }))
    yield result

  def readings(station: String, since: Instant): IO[ProviderError, List[Reading]] =
    for
      memo <- readingsMemo.get
      result <- fresh(memo.get(station)).map(items => Eff.succeed { hits += 1 }.as(items)).getOrElse(
        source.readings(station, since).flatMap(items => readingsMemo.update(_.updated(station, (clock.now, items))).map { _ => misses += 1; items }))
    yield result

  def stats: String = "hits=" + hits + " misses=" + misses
  def invalidate: UIO[Unit] = stationsMemo.set(None) *> readingsMemo.set(Map.empty)

object @P@Cache:
  def over(client: @P@Client, clock: Clock, ttl: Duration = Duration.ofMinutes(5L)): @P@Cache =
    val direct = new @P@Source:
      def stations: IO[ProviderError, List[Station]] = client.stations
      def readings(station: String, since: Instant): IO[ProviderError, List[Reading]] = client.readings(station, since)
    new @P@Cache(direct, ttl, clock)
"""


def write_infra_extras(ctx):
    ctx.write("infra.mail", "Mail", r"""/** Messages to operators: composed, queued and delivered by whatever mailer is wired. */
enum Delivery derives Eq:
  case Queued(id: Int)
  case Sent(id: Int, at: Instant)
  case Bounced(id: Int, reason: String)
  case Suppressed(recipient: String)

final case class Message(
  to: String,
  subject: String,
  body: String,
  cc: List[String] = Nil,
  tags: List[String] = Nil,
  priority: Int = 3
):
  def render: String = "to=" + to + " subject=" + subject + (if cc.isEmpty then "" else " cc=" + cc.mkString(",")) + " p" + priority

trait Mailer:
  def send(message: Message): UIO[Delivery]
  def pending: UIO[Int]

final class QueueMailer(clock: Clock, suppressed: Set[String] = Set.empty) extends Mailer:
  private val queue: Ref[List[(Int, Message)]] = Ref.unsafeMake(Nil)
  private var counter: Int = 0
  def send(message: Message): UIO[Delivery] =
    if suppressed.contains(message.to) then Eff.pure(Delivery.Suppressed(message.to))
    else
      for
        id <- Eff.succeed { counter += 1; counter }
        _ <- queue.update(q => (id, message) :: q)
      yield Delivery.Queued(id)
  def pending: UIO[Int] = queue.get.map(_.length)
  def flush: UIO[List[Delivery]] =
    for
      items <- queue.modify(q => (q.reverse, Nil))
    yield items.map((id, m) => if m.to.contains("@") then Delivery.Sent(id, clock.now) else Delivery.Bounced(id, "no domain"))

object Mailer:
  def describe(delivery: Delivery): String = delivery match
    case Delivery.Queued(id) => "queued #" + id
    case Delivery.Sent(id, at) => "sent #" + id + " at " + at
    case Delivery.Bounced(id, reason) => "bounced #" + id + ": " + reason
    case Delivery.Suppressed(recipient) => "suppressed " + recipient
  def compose(recipient: String, event: Event): Message = event match
    case Event.Created(kind, id, _) => Message(recipient, kind + " created", id, tags = List(kind, "created"))
    case Event.Updated(kind, id, fields, _) => Message(recipient, kind + " updated", fields.mkString(","), tags = List(kind, "updated"), priority = 2)
    case Event.Removed(kind, id, _) => Message(recipient, kind + " removed", id, priority = 1)
    case Event.Custom(kind, id, action, _) => Message(recipient, kind + " " + action, id)
    case Event.Failed(kind, reason, _) => Message(recipient, kind + " failed", reason, priority = 1)
""")
    ctx.write("infra.cache", "Cache", r"""/** A bounded memo keyed by anything, with expiry by the clock. */
trait Cache[K, V]:
  def get(key: K): UIO[Option[V]]
  def put(key: K, value: V): UIO[Unit]
  def getOrLoad(key: K)(load: => UIO[V]): UIO[V] =
    get(key).flatMap {
      case Some(v) => Eff.pure(v)
      case None => load.flatMap(v => put(key, v).as(v))
    }

final class MemoryCache[K, V](clock: Clock, ttl: Duration, capacity: Int = 256) extends Cache[K, V]:
  private val entries: Ref[Map[K, (Instant, V)]] = Ref.unsafeMake(Map.empty)
  def get(key: K): UIO[Option[V]] =
    for
      now <- Eff.succeed(clock.now)
      found <- entries.get.map(_.get(key))
    yield found.filter((at, _) => !at.plus(ttl).isBefore(now)).map(_._2)
  def put(key: K, value: V): UIO[Unit] =
    entries.update { m =>
      val trimmed = if m.size >= capacity then m.toList.sortBy(_._2._1.toEpochMilli).drop(m.size - capacity + 1).toMap else m
      trimmed.updated(key, (clock.now, value))
    }
  def size: UIO[Int] = entries.get.map(_.size)
  def evictExpired: UIO[Int] = entries.modify { m =>
    val kept = m.filter((_, e) => !e._1.plus(ttl).isBefore(clock.now))
    (m.size - kept.size, kept)
  }
""")
    ctx.write("infra.queue", "WorkQueue", r"""/** Work handed between the request path and the executor: enqueued, leased, acknowledged or returned. */
enum Ack:
  case Done, Retry, Drop

final case class Lease[A](id: Int, item: A, attempts: Int, leasedAt: Instant)

final class WorkQueue[A](clock: Clock, maxAttempts: Int = 3):
  private val waiting: Ref[List[(Int, A, Int)]] = Ref.unsafeMake(Nil)
  private val dropped: Ref[List[A]] = Ref.unsafeMake(Nil)
  private var next: Int = 0
  def offer(item: A): UIO[Int] =
    for
      id <- Eff.succeed { next += 1; next }
      _ <- waiting.update(_ :+ ((id, item, 0)))
    yield id
  def offerAll(items: A*): UIO[Int] = Eff.foreach(items.toList)(offer).map(_.length)
  def lease: UIO[Option[Lease[A]]] =
    waiting.modify {
      case (id, item, attempts) :: rest => (Some(Lease(id, item, attempts + 1, clock.now)), rest)
      case Nil => (None, Nil)
    }
  def settle(lease: Lease[A], ack: Ack): UIO[Unit] = ack match
    case Ack.Done => Eff.unit
    case Ack.Retry if lease.attempts < maxAttempts => waiting.update(_ :+ ((lease.id, lease.item, lease.attempts)))
    case Ack.Retry => dropped.update(lease.item :: _)
    case Ack.Drop => dropped.update(lease.item :: _)
  def size: UIO[Int] = waiting.get.map(_.length)
  def droppedCount: UIO[Int] = dropped.get.map(_.length)
  def drain(handle: A => UIO[Ack]): UIO[Int] =
    def loop(done: Int): UIO[Int] =
      lease.flatMap {
        case None => Eff.pure(done)
        case Some(l) => handle(l.item).flatMap(ack => settle(l, ack)).flatMap(_ => loop(done + 1))
      }
    loop(0)
""")


def write_provider(ctx, p):
    rng = ctx.rng
    u = ctx.u
    pkg = f"infra.feeds.{p.lower()}"
    models = []
    with_empty = set()
    caps = {}
    n_models = rng.randint(14, 24)
    lines = [f"/** The wire types of {p}, as its API spells them. */"]
    for i in range(n_models):
        name = f"{p}{u.names.type_name(words=1)}"
        fields = u.names.field_names(rng.randint(2, 9))
        types = [rng.weighted([("String", 5), ("Int", 3), ("Long", 2), ("Boolean", 2), ("Option[String]", 3), ("List[String]", 1), ("Option[Int]", 1), ("Option[Long]", 1)]) for _ in fields]
        other = None
        nestable = [m for m in models if caps[m] & {"JsonCodec", "JsonEncoder", "JsonDecoder"}]
        if i > 2 and nestable and rng.chance(0.3):
            other = rng.pick(nestable)
            fields.append(u.names.field_names(1, extra=fields)[0])
            types.append(rng.pick([other, f"List[{other}]", f"Option[{other}]"]))
        choice = rng.weighted([("JsonCodec", 8), ("JsonCodec, Eq", 2), ("JsonDecoder", 1), ("JsonEncoder", 1), ("", 4)])
        if i == 0:
            choice = "JsonCodec"
        derives = wire_derives(set(choice.split(", ")), caps[other] if other else None) if choice else ""
        caps[name] = set(derives.split(", ")) if derives else set()
        lines.append(f"final case class {name}(")
        for j, (f, t) in enumerate(zip(fields, types)):
            lines.append(f"  {f}: {t}{',' if j < len(fields) - 1 else ''}")
        lines.append(f") derives {derives}" if derives else ")")
        wants_empty = rng.chance(0.3)
        if wants_empty and all(t in with_empty or not t.endswith(m) for t in types for m in [t.strip("List[Option[]")]):
            with_empty.add(name)
            lines.append(f"object {name}:")
            lines.append(f"  def empty: {name} = {name}({', '.join(default_of(t) for t in types)})")
        lines.append("")
        models.append(name)
    enum_name = f"{p}Status"
    cases = u.names.case_names(rng.randint(3, 7))
    lines += [f"enum {enum_name} derives Enumerated, Eq:", "  case " + ", ".join(cases), ""]
    units = [vocab.camel([w]) for w in rng.sample(MEASURES, rng.randint(3, 5))]
    lines += [f"enum {p}Unit derives Enumerated:", "  case " + ", ".join(units), ""]
    ctx.write(pkg, f"{p}Models", "\n".join(lines) + "\n")

    first = models[0]
    client = [f"/** Talks to {p} over a transport of canned answers, decoding its wire types. */",
              f"final case class {p}Settings(baseUrl: String, apiKey: Option[String], timeout: Duration, retries: Int, stations: List[String]) derives JsonCodec",
              f"object {p}Settings:",
              f'  val default: {p}Settings = {p}Settings("https://{p.lower()}.example", None, Duration.ofSeconds(30L), 2, List("a1", "b2", "c3"))',
              "",
              f"final class {p}Client(settings: {p}Settings, transport: Request => Task[Response], clock: Clock) extends Provider:",
              f'  def name = "{p}"',
              f"  private def endpoint(segments: String*): Request = Request(Method.Get, segments.toList, settings.apiKey.map(k => (\"key\", k)).toList, Nil, None)",
              f"  private def decode[A](response: Response)(using d: JsonDecoder[A]): IO[ProviderError, A] =",
              f"    response.status match",
              f'      case 200 => Eff.fromEither(response.body.toRight("empty").flatMap(d.decodeJson)).mapError(e => ProviderError.Malformed(name, e))',
              f"      case 401 | 403 => Eff.fail(ProviderError.Unauthorised(name))",
              f"      case 429 => Eff.fail(ProviderError.RateLimited(name, settings.timeout))",
              f"      case _ => Eff.fail(ProviderError.Unreachable(name))",
              f"  private def call[A](request: Request)(using d: JsonDecoder[A]): IO[ProviderError, A] =",
              f"    transport(request).mapError(_ => ProviderError.Unreachable(name)).flatMap(decode[A]).retry(settings.retries)",
              f"  def stations: IO[ProviderError, List[Station]] =",
              f"    call[List[{first}]](endpoint(\"stations\")).map(_.zipWithIndex.map((raw, i) => {p}Mapping.station(raw, i)))",
              f"  def readings(station: String, since: Instant): IO[ProviderError, List[Reading]] =",
              f"    call[List[{first}]](endpoint(\"stations\", station, \"readings\")).map(_.map(raw => {p}Mapping.reading(raw, station, clock.now)).filter(_.at.isAfter(since) || true))",
              f"  def health: UIO[Boolean] = transport(endpoint(\"health\")).map(_.status == 200).catchAll(_ => Eff.pure(false))",
              f"  def describe: String = name + \"@\" + settings.baseUrl",
              "",
              f"object {p}Client:",
              f"  given cats.Show[{p}Settings] = s => s.baseUrl + \" retries=\" + s.retries",
              f"  given cats.Show[{enum_name}] = s => s.entryName.toLowerCase",
              f"  def canned(clock: Clock): {p}Client =",
              f"    new {p}Client({p}Settings.default, request => Eff.succeed({p}Fixtures.answer(request)), clock)"]
    ctx.write(pkg, f"{p}Client", "\n".join(client) + "\n", extra="import meridian.infra.feeds.*")

    mapping = [f"/** From {p}'s wire types to the readings and stations the services use. */", f"object {p}Mapping:",
               f"  def station(raw: {first}, index: Int): Station = Station(\"{p}\", raw.productElement(0).toString, raw.productPrefix + index, index % 3 != 2)",
               f"  def reading(raw: {first}, station: String, at: Instant): Reading =",
               f"    val fields = raw.productIterator.toList",
               f"    val value = fields.collectFirst {{ case i: Int => i.toLong; case l: Long => l }}.getOrElse(fields.length.toLong)",
               f"    val flags = fields.collect {{ case b: Boolean if b => \"flagged\"; case Some(s: String) => s.take(8) }}",
               f"    Reading(\"{p}\", station, at, value, unit(value), flags)",
               f"  def unit(value: Long): String = unitOf(value).entryName.toLowerCase",
               f"  def unitOf(value: Long): {p}Unit = value match",
               f"    case v if v < 0 => {p}Unit.{units[0]}",
               f"    case 0 => {p}Unit.{units[1]}",
               f"    case v if v < 100 => {p}Unit.{units[2]}",
               f"    case _ => {p}Unit.{units[-1]}",
               f"  def status(healthy: Boolean, readings: Int): {enum_name} = (healthy, readings) match",
               f"    case (false, _) => {enum_name}.{cases[0]}",
               f"    case (true, 0) => {enum_name}.{cases[1]}",
               f"    case _ => {enum_name}.{cases[-1]}",
               f"  def merge(a: Reading, b: Reading): Reading = a.copy(value = (a.value + b.value) / 2, flags = (a.flags ++ b.flags).distinct)",
               f"  def latest(readings: List[Reading]): Option[Reading] = readings.sortBy(_.at.toEpochMilli).lastOption"]
    ctx.write(pkg, f"{p}Mapping", "\n".join(mapping) + "\n", extra="import meridian.infra.feeds.*")

    fixtures = [f"/** What the canned {p} answers with, by request path. */", f"object {p}Fixtures:"]
    for m in models[:6]:
        fixtures.append(f"  val {lower(m)}: {m} = {m}({sample_of(ctx, pkg, m)})")
    fixtures += [f"  val rawStations: String = \"\"\"[", raw_json(ctx, pkg, first, rng), "  ]\"\"\"",
                 f"  def decodedStations: Either[String, List[{first}]] = rawStations.fromJson[List[{first}]]"]
    fixtures += [f"  def answer(request: Request): Response = request.segments match",
                 f'    case List("stations") => Response(200, Nil, Some(List({lower(first)}, {lower(first)}).toJson))',
                 f'    case List("stations", _, "readings") => Response(200, Nil, Some(List({lower(first)}).toJson))',
                 f'    case List("health") => Response(200, Nil, None)',
                 f'    case List("stations", "x9") => Response(404, Nil, None)',
                 f'    case _ => Response(500, Nil, None)']
    ctx.write(pkg, f"{p}Fixtures", "\n".join(fixtures) + "\n", extra="import meridian.infra.feeds.*")

    health = [f"/** The health of {p}: what the last checks said and how they are summarised. */",
              f"final case class {p}Check(at: Instant, healthy: Boolean, latencyMillis: Long, note: Option[String]) derives JsonCodec",
              f"final class {p}Health(window: Int):",
              "  private var checks: List[" + p + "Check] = Nil",
              f"  def record(check: {p}Check): UIO[Unit] = Eff.succeed {{ checks = (check :: checks).take(window) }}",
              f"  def uptime: Int = if checks.isEmpty then 100 else checks.count(_.healthy) * 100 / checks.length",
              f"  def status: {enum_name} = {p}Mapping.status(uptime > 50, checks.length)",
              f"  def latest: Option[{p}Check] = checks.headOption",
              f"  def summary: String = status match"]
    for c in cases:
        health.append(f"    case {enum_name}.{c} => \"{c.lower()} (\" + uptime + \"%)\"")
    health += ["", f"object {p}Health:",
               f"  def probe(client: {p}Client, clock: Clock, health: {p}Health): UIO[Boolean] =",
               "    for", "      healthy <- client.health", f"      _ <- health.record({p}Check(clock.now, healthy, if healthy then 12L else 900L, Option.when(!healthy)(\"probe failed\")))", "    yield healthy"]
    ctx.write(pkg, f"{p}Health", "\n".join(health) + "\n", extra="import meridian.infra.feeds.*")
    ctx.write(pkg, f"{p}Sync", SYNC.replace("@P@", p), extra="import meridian.infra.feeds.*")
    ctx.write(pkg, f"{p}Cache", CACHE.replace("@P@", p), extra="import meridian.infra.feeds.*")


def default_of(t):
    return {"String": '""', "Int": "0", "Long": "0L", "Boolean": "false"}.get(t, "None" if t.startswith("Option") else "Nil" if t.startswith("List") else t + ".empty")


def sample_of(ctx, pkg, model_name):
    """Constructor arguments for a wire model, read back from the file just written."""
    path = os.path.join(ctx.out, *pkg.split("."), pkg.split(".")[-1].capitalize())
    text = open(os.path.join(ctx.out, *pkg.split("."), [f for f in os.listdir(os.path.join(ctx.out, *pkg.split("."))) if f.endswith("Models.scala")][0])).read()
    start = text.index(f"final case class {model_name}(")
    end = text.index(")", start)
    body = text[start + len(f"final case class {model_name}("): end]
    args = []
    rng = ctx.rng
    for line in body.split("\n"):
        line = line.strip().rstrip(",")
        if not line:
            continue
        name, tpe = line.split(": ", 1)
        if tpe == "String":
            args.append(f'"{rng.pick(vocab.NOUNS)}"')
        elif tpe == "Int":
            args.append(str(rng.randint(0, 900)))
        elif tpe == "Long":
            args.append(str(rng.randint(0, 90000)) + "L")
        elif tpe == "Boolean":
            args.append(rng.pick(["true", "false"]))
        elif tpe == "Option[String]":
            args.append(f'Some("{rng.pick(vocab.QUALIFIERS)}")')
        elif tpe in ("Option[Int]", "Option[Long]"):
            args.append("None")
        elif tpe == "List[String]":
            args.append(f'List("{rng.pick(vocab.NOUNS)}")')
        elif tpe.startswith("List["):
            args.append("Nil")
        elif tpe.startswith("Option["):
            args.append("None")
        else:
            args.append(f"{lower(tpe)}")
    return ", ".join(args)


# ---------------------------------------------------------------------------------------------
# business

def write_business_area(ctx, area, decls, entities):
    entity = entities[0] if entities else None
    if entity is None:
        write_plain_service(ctx, area, decls)
        return
    write_queries(ctx, area, decls, entity)
    write_repo(ctx, area, decls, entity)
    write_rules(ctx, area, decls, entity)
    write_audit(ctx, area, decls, entity)
    write_service(ctx, area, decls, entity)
    write_reports(ctx, area, decls, entity)
    write_export(ctx, area, decls, entity)
    write_notifications(ctx, area, decls, entity)


def write_plain_service(ctx, area, decls):
    enums = [d for d in decls if d.kind == "enum" and d.has("Enumerated")]
    e0 = enums[0].name if enums else None
    lines = [f"/** The {area.lower()} area has no entity of its own: a service over notes and events. */", f"object {area}Service:",
             f"  final case class Note(text: String, at: Instant, tags: List[String])",
             f"  private val notes: Ref[List[Note]] = Ref.unsafeMake(Nil)",
             f"  def add(text: String, tags: List[String]): Eff[Clock & Bus & Logger, ServiceError, Note] =",
             "    for", "      _ <- Eff.cond(text.trim.nonEmpty, (), ServiceError.Invalid(\"empty note\"))", "      clock <- Eff.service[Clock]",
             "      note = Note(text.trim, clock.now, tags.distinct)", "      _ <- notes.update(note :: _)",
             f"      _ <- Eff.service[Bus].flatMap(_.publish(Event.Created(\"{area.lower()}\", text.take(8), clock.now)))",
             f"      _ <- scoped(\"{area.lower()}\").info(\"note added\", \"tags\" -> tags.length.toString)", "    yield note",
             "  def all: UIO[List[Note]] = notes.get.map(_.reverse)",
             "  def tagged(tag: String): UIO[List[Note]] = all.map(_.filter(_.tags.contains(tag)))",
             "  def clear: UIO[Int] = notes.modify(ns => (ns.length, Nil))",
             "  def summary: UIO[String] = all.map(ns => ns.length match", "    case 0 => \"no notes\"", "    case 1 => \"one note\"", "    case n => s\"$n notes\")"]
    if e0:
        lines += [f"  def label(value: {e0}): String = value.entryName.toUpperCase"]
    ctx.write(f"business.{area.lower()}", f"{area}Service", "\n".join(lines) + "\n", extra="import meridian.business.*")


def entity_slots(entity):
    """The fields the area's queries and reports pick on, by role."""
    fields = entity.fields
    return {
        "status": next(((n, t) for n, t in fields if t.kind == "enum" and t.text), None),
        "texts": [n for n, t in fields if t.expr == "String"][:2],
        "numbers": [(n, t.expr) for n, t in fields if t.expr in ("Int", "Long")][:2],
        "bools": [n for n, t in fields if t.expr == "Boolean"][:1],
        "dates": [n for n, t in fields if t.expr in ("Instant", "LocalDate")][:1],
    }


def write_repo(ctx, area, decls, entity):
    en = entity.name
    idn = entity.entity_id.name
    table = area.lower()
    slots = entity_slots(entity)
    status, texts, numbers, bools = slots["status"], slots["texts"], slots["numbers"], slots["bools"]
    Q = f"{area}Queries"
    columns = ["id"] + [n for n, _ in entity.fields if n != "id"]
    signatures = [(f"find(id: {idn})", f"Task[Option[{en}]]"), ("all(limit: Int = 50)", f"Task[List[{en}]]"),
                  (f"insert(item: {en})", f"Task[{en}]"), (f"update(item: {en})", "Task[Boolean]"),
                  (f"delete(id: {idn})", "Task[Boolean]"), ("count", "Task[Int]"),
                  (f"byIds(ids: List[{idn}])", f"Task[List[{en}]]"), (f"query(listing: {Q}.Listing)", f"Task[List[{en}]]"),
                  ("page(offset: Int = 0, size: Int = 50)", f"Task[List[{en}]]"), (f"upsertAll(items: List[{en}])", "Task[Int]"),
                  (f"replace(items: {en}*)", "Task[Int]"), ("statements", "List[String]")]
    if status:
        signatures.append((f"byStatus(value: {status[1].expr})", f"Task[List[{en}]]"))
    for n in texts:
        signatures.append((f"search{cap(n)}(text: String, limit: Int = 20)", f"Task[List[{en}]]"))
    for n, _ in numbers:
        signatures.append((f"{n}Above(threshold: Long)", f"Task[List[{en}]]"))
    for n in bools:
        signatures.append((f"{n}Only", f"Task[List[{en}]]"))
    lines = [f"/** The {table} table as the services see it: every access an SQL fragment, run by the transactor. */",
             f"trait {area}Repo:"]
    for sig, ret in signatures:
        lines.append(f"  def {sig}: {ret}")
    lines += ["", f"object {area}Repo:", f"  def seeded(items: List[{en}], xa: Transactor): {area}Repo = new Live{area}Repo(Table(items, _.id), xa)",
              f"  def empty(xa: Transactor): {area}Repo = seeded(Nil, xa)",
              f"  val columns: List[String] = List(", "    " + ",\n    ".join(f'"{c}"' for c in columns), "  )", "",
             f"/** The live {table} repository over the in-memory table, keeping the SQL it would send. */",
             f"final class Live{area}Repo(table: Table[{idn}, {en}], xa: Transactor) extends {area}Repo:",
             f"  private val columns: Fragment = Fragment.const({area}Repo.columns.mkString(\", \"))",
             f"  private val select: Fragment = sql\"\"\"", f"    select $columns", f"    from {table}", "    \"\"\"",
             f"  private val countAll: Fragment = sql\"\"\"", f"    select count(*)", f"    from {table}", "    \"\"\"",
             "",
             f"  def find(id: {idn}): Task[Option[{en}]] =",
             f"    xa.one((select ++ sql\"\"\"", f"      where id = {bound(ctx, 'id')}", f"      \"\"\").query[{en}])(table.get(id))",
             f"  def all(limit: Int): Task[List[{en}]] =",
             f"    xa.run(select.orderBy(\"id\").limit(limit).query[{en}])(table.all.map(_.sortBy(_.id.raw).take(limit)))",
             f"  def insert(item: {en}): Task[{en}] =",
             f"    xa.execute(sql\"\"\"", f"      insert into {table} ({', '.join(columns)})", f"      values ({bound(ctx, 'item.id')}, {', '.join('?' for _ in columns[1:])})", f"      \"\"\".update)(table.put(item).as(1)).as(item)",
             f"  def update(item: {en}): Task[Boolean] =",
             f"    xa.execute(sql\"\"\"", f"      update {table}", "      set " + ", ".join(f"{c} = ?" for c in columns[1:]), f"      where id = {bound(ctx, 'item.id')}", f"      \"\"\".update)(table.get(item.id).flatMap(existing => Eff.when(existing.isDefined)(table.put(item)).as(if existing.isDefined then 1 else 0))).map(_ > 0)",
             f"  def delete(id: {idn}): Task[Boolean] =",
             f"    xa.execute(sql\"\"\"", f"      delete from {table}", f"      where id = {bound(ctx, 'id')}", f"      \"\"\".update)(table.remove(id).map(removed => if removed then 1 else 0)).map(_ > 0)",
             f"  def count: Task[Int] =",
             f"    xa.run(countAll.query[Int])(table.size.map(n => List(n))).map(_.headOption.getOrElse(0))",
             f"  def byIds(ids: List[{idn}]): Task[List[{en}]] =",
             f"    xa.run((select ++ fr\"where id in ${{Fragment.in(ids)}}\").query[{en}])(table.where(item => ids.contains(item.id)))",
             f"  def query(listing: {Q}.Listing): Task[List[{en}]] =",
             f"    xa.run({Q}.fragment(select, listing).query[{en}])(table.all.map(items => {Q}(items, listing)))"]
    if status:
        sn, st = status[0], status[1].expr
        lines += [f"  def byStatus(value: {st}): Task[List[{en}]] =",
                  f"    xa.run((select ++ sql\"\"\"", f"      where {sn} = {bound(ctx, 'value')}", "      order by id", f"      \"\"\").query[{en}])(table.where(_.{sn} == value))"]
    for n in texts:
        lines += [f"  def search{cap(n)}(text: String, limit: Int): Task[List[{en}]] =",
                  '    val pattern = "%" + text.toLowerCase + "%"',
                  f"    xa.run((select ++ sql\"\"\"", f"      where lower({n}) like $pattern", "      order by id", f"      \"\"\").limit(limit).query[{en}])(table.where(_.{n}.toLowerCase.contains(text.toLowerCase)).map(_.take(limit)))"]
    for n, _ in numbers:
        lines += [f"  def {n}Above(threshold: Long): Task[List[{en}]] =",
                  f"    xa.run((select ++ sql\"\"\"", f"      where {n} > $threshold", f"      \"\"\").query[{en}])(table.where(_.{n}.toLong > threshold))"]
    for n in bools:
        lines += [f"  def {n}Only: Task[List[{en}]] =",
                  f"    xa.run((select ++ sql\"\"\"", f"      where {n} = ${{true}}", f"      \"\"\").query[{en}])(table.where(_.{n}))"]
    lines += [f"  def page(offset: Int, size: Int): Task[List[{en}]] =",
              f"    xa.run((select.orderBy(\"id\") ++ sql\"offset $offset\").limit(size).query[{en}])(table.all.map(_.sortBy(_.id.raw).drop(offset).take(size)))",
              f"  def upsertAll(items: List[{en}]): Task[Int] =",
              "    xa.transactionally(Eff.foreach(items)(insert).map(_.length))",
              f"  def replace(items: {en}*): Task[Int] =",
              "    xa.transactionally(",
              "      for",
              f"        current <- table.all",
              f"        _ <- Eff.foreachDiscard(current)(item => table.remove(item.id))",
              f"        _ <- xa.execute(sql\"\"\"", f"          delete from {table}", "          \"\"\".update)(Eff.pure(current.length))",
              "        inserted <- Eff.foreach(items.toList)(insert)",
              "      yield inserted.length",
              "    )",
              f"  def statements: List[String] = xa.seen.filter(_.contains(\"{table}\"))"]
    ctx.write(f"business.{area.lower()}", f"{area}Repo", "\n".join(lines) + "\n", extra="import meridian.business.*")


def cap(name):
    return name[0].upper() + name[1:]


def write_queries(ctx, area, decls, entity):
    en = entity.name
    idn = entity.entity_id.name
    slots = entity_slots(entity)
    status, texts, numbers, bools = slots["status"], slots["texts"], slots["numbers"], slots["bools"]
    text = texts[0] if texts else None
    number = numbers[0] if numbers else None
    flag = bools[0] if bools else None
    filter_fields = [("text", "Option[String]", "None")]
    if status:
        filter_fields.append((status[0], f"Option[{status[1].expr}]", "None"))
    if number:
        filter_fields.append((f"min{cap(number[0])}", "Option[Long]", "None"))
    if flag:
        filter_fields.append((flag, "Option[Boolean]", "None"))
    filter_fields.append(("ids", f"List[{idn}]", "Nil"))
    sort_fields = ["id"] + ([text] if text else []) + ([number[0]] if number else [])
    lines = [f"/** How {area.lower()} listings are asked for: criteria, a page and an order, as SQL and as a predicate. */",
             f"object {area}Queries:",
             "  final case class Criteria("]
    lines.append(",\n".join(f"    {n}: {t} = {d}" for n, t, d in filter_fields))
    lines += ["  ):",
              "    def isEmpty: Boolean = this == Criteria()",
              "    def withText(value: String): Criteria = copy(text = Some(value).filter(_.nonEmpty))",
              "",
              "  final case class Listing(",
              "    criteria: Criteria = Criteria(),",
              "    offset: Int = 0,",
              "    size: Int = 50,",
              "    sort: String = \"id\",",
              "    descending: Boolean = false",
              "  ):",
              "    def page(n: Int): Listing = copy(offset = n * size)",
              "    def next: Listing = copy(offset = offset + size)",
              "    def reversed: Listing = copy(descending = !descending)",
              "    def pageNumber: Int = offset / Math.max(size, 1)",
              "",
              "  val default: Listing = Listing()",
              f"  val sortable: List[String] = List({', '.join(f'\"{f}\"' for f in sort_fields)})",
              "",
              "  def conditions(criteria: Criteria): List[Fragment] =",
              "    List("]
    conds = [f"      criteria.text.map(t => sql\"\"\"lower({text}) like ${{\"%\" + t.toLowerCase + \"%\"}}\"\"\")" if text else "      criteria.text.map(t => sql\"\"\"cast(id as text) like ${\"%\" + t + \"%\"}\"\"\")"]
    if status:
        conds.append(f"      criteria.{status[0]}.map(s => sql\"\"\"{status[0]} = {bound(ctx, 's')}\"\"\")")
    if number:
        conds.append(f"      criteria.min{cap(number[0])}.map(m => sql\"\"\"{number[0]} >= $m\"\"\")")
    if flag:
        conds.append(f"      criteria.{flag}.map(f => sql\"\"\"{flag} = $f\"\"\")")
    conds.append("      Option.when(criteria.ids.nonEmpty)(fr\"id in ${Fragment.in(criteria.ids)}\")")
    lines.append(",\n".join(conds))
    lines += ["    ).flatten", "",
              "  def order(listing: Listing): Fragment =",
              "    val column = listing.sort match"]
    for f in sort_fields[1:]:
        lines.append(f"      case \"{f}\" => \"{f}\"")
    lines += ["      case _ => \"id\"",
              "    Fragment.const(\"order by \" + column + (if listing.descending then \" desc\" else \" asc\"))",
              "",
              "  def fragment(select: Fragment, listing: Listing): Fragment =",
              "    val where = conditions(listing.criteria) match",
              "      case Nil => Fragment.empty",
              "      case conds => Fragment.const(\"where\") ++ Fragment.and(conds)",
              "    (select ++ where ++ order(listing) ++ sql\"offset ${listing.offset}\").limit(listing.size)",
              "",
              f"  def matches(item: {en}, criteria: Criteria): Boolean ="]
    preds = [f"    criteria.text.forall(t => item.{text}.toLowerCase.contains(t.toLowerCase))" if text else "    criteria.text.forall(t => item.id.raw.toString.contains(t))"]
    if status:
        preds.append(f"    criteria.{status[0]}.forall(_ == item.{status[0]})")
    if number:
        preds.append(f"    criteria.min{cap(number[0])}.forall(item.{number[0]}.toLong >= _)")
    if flag:
        preds.append(f"    criteria.{flag}.forall(_ == item.{flag})")
    preds.append("    (criteria.ids.isEmpty || criteria.ids.contains(item.id))")
    lines.append(" &&\n".join(preds))
    lines += ["",
              f"  def sort(items: List[{en}], listing: Listing): List[{en}] =",
              "    val sorted = listing.sort match"]
    for f in sort_fields[1:]:
        lines.append(f"      case \"{f}\" => items.sortBy(_.{f})")
    lines += ["      case _ => items.sortBy(_.id.raw)",
              "    if listing.descending then sorted.reverse else sorted",
              "",
              f"  def apply(items: List[{en}], listing: Listing): List[{en}] =",
              "    sort(items.filter(matches(_, listing.criteria)), listing).drop(listing.offset).take(listing.size)",
              "",
              "  def fromParams(params: Map[String, String]): Listing =",
              "    Listing(",
              "      criteria = Criteria(",
              "        text = params.get(\"q\").filter(_.nonEmpty)" + ("," if len(filter_fields) > 2 else "")]
    extra = []
    if status:
        extra.append(f"        {status[0]} = params.get(\"{status[0]}\").flatMap(Enumerated[{status[1].expr}].valueOf)")
    if number:
        extra.append(f"        min{cap(number[0])} = params.get(\"min\").flatMap(_.toLongOption)")
    if flag:
        extra.append(f"        {flag} = params.get(\"{flag}\").map(_ == \"true\")")
    if extra:
        lines.append(",\n".join(extra))
    lines += ["      ),",
              "      offset = params.get(\"offset\").flatMap(_.toIntOption).getOrElse(0),",
              "      size = params.get(\"size\").flatMap(_.toIntOption).getOrElse(50),",
              "      sort = params.getOrElse(\"sort\", \"id\"),",
              "      descending = params.get(\"dir\").contains(\"desc\")",
              "    )",
              "",
              "  def describe(listing: Listing): String =",
              "    val parts = List(",
              "      listing.criteria.text.map(\"text=\" + _),",
              "      Option.when(listing.criteria.ids.nonEmpty)(\"ids=\" + listing.criteria.ids.length),",
              "      Some(\"page=\" + listing.pageNumber),",
              "      Some(\"sort=\" + listing.sort + (if listing.descending then \"-\" else \"+\"))",
              "    ).flatten",
              "    parts.mkString(\" \")"]
    ctx.write(f"business.{area.lower()}", f"{area}Queries", "\n".join(lines) + "\n", extra="import meridian.business.*")


def write_audit(ctx, area, decls, entity):
    rng = ctx.rng
    en = entity.name
    idn = entity.entity_id.name
    fields = [n for n, _ in entity.fields if n != "id"][:8]
    lines = [f"/** What changed on a {en}, by whom and when, kept in a bounded trail. */",
             f"object {area}Audit:",
             "  final case class Change(field: String, before: String, after: String):",
             "    def render: String = s\"$field: $before -> $after\"",
             "",
             f"  final case class Entry(id: {idn}, actor: String, changes: List[Change], at: Instant, note: Option[String] = None):",
             "    def isEmpty: Boolean = changes.isEmpty && note.isEmpty",
             "    def fields: List[String] = changes.map(_.field)",
             "",
             f"  def changes(a: {en}, b: {en}): List[Change] ="]
    parts = [f"    (if a.{n} != b.{n} then List(Change(\"{n}\", a.{n}.toString, b.{n}.toString)) else Nil)" for n in fields]
    lines.append(" ++\n".join(parts) if parts else "    Nil")
    lines += ["",
              f"  def entry(id: {idn}, actor: String, changes: List[Change], at: Instant, note: Option[String] = None): Entry =",
              "    Entry(id = id, actor = actor, changes = changes, at = at, note = note)",
              "",
              "  def render(entry: Entry): String =",
              "    s\"\"\"${entry.actor} @ ${entry.at}",
              "       |  item ${entry.id.raw}",
              "       |  ${entry.changes.map(_.render).mkString(\"; \")}",
              "       |  ${entry.note.getOrElse(\"\")}\"\"\".stripMargin.trim",
              "",
              "  def significant(entry: Entry): Boolean = entry.fields match",
              "    case Nil => entry.note.isDefined",
              f"    case List(single) => single != \"{fields[-1] if fields else 'id'}\"",
              "    case many => many.length > 1",
              "",
              "  def summarise(entries: List[Entry]): Map[String, Int] =",
              "    entries.flatMap(_.fields).groupBy(identity).map((field, occurrences) => (field, occurrences.length))",
              "",
              f"  def latest(entries: List[Entry], id: {idn}): Option[Entry] =",
              "    entries.filter(_.id == id).sortBy(_.at.toEpochMilli).lastOption",
              "",
              "  def actors(entries: List[Entry]): List[String] = entries.map(_.actor).distinct.sorted",
              "",
              f"/** The trail: the last `limit` entries, newest first. */",
              f"final class {area}Trail(limit: Int = 100):",
              f"  private val ref: Ref[List[{area}Audit.Entry]] = Ref.unsafeMake(Nil)",
              f"  def record(entry: {area}Audit.Entry): UIO[Unit] = ref.update(entries => (entry :: entries).take(limit))",
              f"  def forItem(id: {idn}): UIO[List[{area}Audit.Entry]] = ref.get.map(_.filter(_.id == id))",
              f"  def all: UIO[List[{area}Audit.Entry]] = ref.get",
              "  def count: UIO[Int] = ref.get.map(_.length)",
              "  def prune(before: Instant): UIO[Int] = ref.modify { entries =>",
              "    val kept = entries.filter(e => !e.at.isBefore(before))",
              "    (entries.length - kept.length, kept)",
              "  }",
              f"  def byActor: UIO[Map[String, Int]] = ref.get.map(_.groupBy(_.actor).map((actor, entries) => (actor, entries.length)))"]
    if rng.chance(0.6):
        lines += [f"  def unsafeLatest(id: {idn}): UIO[{area}Audit.Entry] =",
                  f"    forItem(id).map(entries => {area}Audit.latest(entries, id).getOrElse(throw new NoSuchElementException(s\"no trail for ${{id.raw}}\")))"]
    ctx.write(f"business.{area.lower()}", f"{area}Audit", "\n".join(lines) + "\n", extra="import meridian.business.*")


def write_rules(ctx, area, decls, entity):
    rng = ctx.rng
    en = entity.name
    idn = entity.entity_id.name
    texts = [n for n, t in entity.fields if t.expr == "String"][:3]
    numbers = [n for n, t in entity.fields if t.expr in ("Int", "Long")][:2]
    options = [(n, t) for n, t in entity.fields if t.kind == "option"][:2]
    lists = [n for n, t in entity.fields if t.kind == "list"][:1]
    status = next(((n, t) for n, t in entity.fields if t.kind == "enum"), None)
    enum_decl = next((d for d in decls if status and d.name == status[1].expr), None)
    others = [d for d in decls if d.kind == "case" and d is not entity and "JsonCodec" in d.derives]
    lines = [f"/** What a {en} has to satisfy, and how it maps to and from its related types. */", f"object {area}Rules:",
             *([] if ctx.flags.std_members else [f"  private val fieldNames: List[String] = summon[Schema[{en}]].fieldNames"]),
             f"  def validate(item: {en}): Check[{en}] =", "    Check.all(List("]
    checks = [f'      Rules.positive("id", {"item.id.raw" if entity.entity_id.raw_type == "Long" else "item.id.raw.length.toLong"}).void']
    for n in texts:
        checks.append(f'      Rules.nonEmpty("{n}", item.{n}).void')
        checks.append(f'      Rules.maxLength("{n}", item.{n}, 200).void')
    for n in numbers:
        checks.append(f'      Rules.nonNegative("{n}", item.{n}.toLong).void')
    for n in lists:
        checks.append(f'      Check.cond(item.{n}.length <= 100, (), "{n} has too many entries")')
    lines.append(",\n".join(checks))
    lines += ["    )).as(item)", "",
              f"  def isValid(item: {en}): Boolean = validate(item).isValid", ""]
    if status and enum_decl:
        first, last = enum_decl.cases[0], enum_decl.cases[-1]
        lines += [f"  def canTransition(from: {enum_decl.name}, to: {enum_decl.name}): Boolean = (from, to) match",
                  f"    case (a, b) if a == b => false",
                  f"    case ({enum_decl.name}.{last}, _) => false",
                  f"    case (_, {enum_decl.name}.{first}) => false",
                  "    case _ => true",
                  f"  def transition(item: {en}, to: {enum_decl.name}): Either[ServiceError, {en}] =",
                  f"    if canTransition(item.{status[0]}, to) then Right(item.copy({status[0]} = to))",
                  f"    else Left(ServiceError.Conflict(s\"cannot move ${{item.{status[0]}}} to $to\"))",
                  f"  def isTerminal(item: {en}): Boolean = item.{status[0]} == {enum_decl.name}.{last}", ""]
    lines += [f"  def merge(current: {en}, incoming: {en}): {en} ="]
    merges = ["current.copy("]
    parts = []
    for n in texts[:2]:
        parts.append(f"{n} = if incoming.{n}.isEmpty then current.{n} else incoming.{n}")
    for n, t in options:
        parts.append(f"{n} = incoming.{n}.orElse(current.{n})")
    for n in lists:
        parts.append(f"{n} = (current.{n} ++ incoming.{n}).distinct")
    lines.append("    " + ("current.copy(" + ", ".join(parts) + ")" if parts else "incoming"))
    lines += ["", f"  def diff(a: {en}, b: {en}): List[String] =",
              "    a.productIterator.zip(b.productIterator).zipWithIndex.collect { case ((x, y), i) if x != y => " + ("a.productElementName(i)" if ctx.flags.std_members else "fieldNames(i)") + " }.toList",
              f"  def summary(item: {en}): String = " + (ctx.flags.show_interpolator and 'show"' or 's"') + "${item.id.raw}" + ("".join(f" ${{item.{n}}}" for n in texts[:1])) + '"']
    if status and enum_decl:
        lines += ["", f"  def describe(item: {en}): String = item.{status[0]} match"]
        for case in enum_decl.cases[:7]:
            lines.append(f"    case {enum_decl.name}.{case} => \"{rng.pick(vocab.QUALIFIERS)} {case.lower()}\"")
        if len(enum_decl.cases) > 7:
            lines.append("    case other => other.entryName")
    elif numbers:
        lines += ["", f"  def describe(item: {en}): String = item.{numbers[0]}.toLong match",
                  "    case 0L => \"none\"", "    case n if n < 10L => \"a few\"", "    case n if n < 100L => \"some\"", "    case n if n < 1000L => \"many\"", "    case _ => \"plenty\""]
    else:
        lines += ["", f"  def describe(item: {en}): String = item.id.raw.toString.length match",
                  "    case 1 => \"short\"", "    case 2 | 3 => \"medium\"", "    case _ => \"long\""]
    if rng.chance(0.4):
        lines += ["", f"  enum Priority derives Enumerated:", "    case Low, Normal, High, Urgent",
                  f"  def priority(item: {en}): Priority ="]
        if numbers:
            lines += [f"    item.{numbers[0]}.toLong match", "      case n if n <= 0L => Priority.Low", "      case n if n < 50L => Priority.Normal", "      case n if n < 500L => Priority.High", "      case _ => Priority.Urgent"]
        else:
            lines += [f"    if isValid(item) then Priority.Normal else Priority.High"]
    if others:
        o = others[0]
        lines += ["", f"  final case class {en}View(", f"    id: {idn},", "    label: String,", "    valid: Boolean,", f"    related: Option[{o.name}] = None", "  ) derives JsonCodec",
                  f"  def view(item: {en}, related: Option[{o.name}]): {en}View = {en}View(item.id, summary(item), isValid(item), related)"]
    else:
        lines += ["", f"  final case class {en}View(", f"    id: {idn},", "    label: String,", "    valid: Boolean", "  ) derives JsonCodec",
                  f"  def view(item: {en}): {en}View = {en}View(item.id, summary(item), isValid(item))"]
    ctx.write(f"business.{area.lower()}", f"{area}Rules", "\n".join(lines) + "\n", extra="import meridian.business.*")


def write_service(ctx, area, decls, entity):
    rng = ctx.rng
    en = entity.name
    idn = entity.entity_id.name
    R = f"{area}Repo"
    items = ctx.by_area[area]
    customs = [e for e in items if e.action == "custom"]
    status = next(((n, t) for n, t in entity.fields if t.kind == "enum"), None)
    enum_decl = next((d for d in decls if status and d.name == status[1].expr), None)
    kind = area.lower()
    env = f"{R} & Clock & Bus & Logger"
    lines = [f"/** The {kind} operations behind the routes: each one loads, checks, writes, and tells the bus. */",
             f"object {area}Service:", f"  type Op[A] = Eff[{env}, ServiceError, A]",
             f"  private val log = scoped(\"{kind}\")",
             f"  private def repo: Eff[{R}, Nothing, {R}] = Eff.service[{R}]",
             f"  private def now: Eff[Clock, Nothing, Instant] = Eff.service[Clock].map(_.now)",
             f"  private def publish(event: Event): Eff[Bus, Nothing, Unit] = Eff.service[Bus].flatMap(_.publish(event))",
             f"  private def attempt[A](task: Task[A]): IO[ServiceError, A] = task.mapError(t => ServiceError.Failure(t.getMessage))",
             "",
             f"  def get(id: {idn}): Op[{en}] =", "    for", "      r <- repo", "      found <- attempt(r.find(id))",
             f"      item <- Eff.fromOption(found).mapError(_ => ServiceError.NotFound(\"{kind}\", id.raw.toString))",
             f"      _ <- log.debug(\"get \" + id.raw)", "    yield item", "",
             f"  def list(limit: Option[Int]): Op[List[{en}]] =", "    for", "      r <- repo", "      items <- attempt(r.all(limit.getOrElse(50)))",
             f"      _ <- log.info(\"list\", \"count\" -> items.length.toString)", "    yield items", "",
             f"  def create(item: {en}): Op[{en}] =", "    for", "      r <- repo", f"      valid <- Eff.fromEither({area}Rules.validate(item).toEither).mapError(fs => ServiceError.Invalid(fs.map(_.render).mkString(\"; \")))",
             f"      existing <- attempt(r.find(item.id))", f"      _ <- Eff.when(existing.isDefined)(Eff.fail(ServiceError.Conflict(\"{kind} \" + item.id.raw + \" exists\")))",
             "      saved <- attempt(r.insert(valid))", "      at <- now", f"      _ <- publish(Event.Created(\"{kind}\", item.id.raw.toString, at))",
             "    yield saved", "",
             f"  def update(id: {idn}, incoming: {en}): Op[Unit] =", "    for", "      current <- get(id)", f"      merged = {area}Rules.merge(current, incoming)",
             f"      valid <- Eff.fromEither({area}Rules.validate(merged).toEither).mapError(fs => ServiceError.Invalid(fs.map(_.render).mkString(\"; \")))",
             "      r <- repo", "      _ <- attempt(r.update(valid))", "      at <- now",
             f"      _ <- publish(Event.Updated(\"{kind}\", id.raw.toString, {area}Rules.diff(current, valid), at))", "    yield ()", "",
             f"  def remove(id: {idn}): Op[Unit] =", "    for", "      r <- repo", "      removed <- attempt(r.delete(id))",
             f"      _ <- Eff.unless(removed)(Eff.fail(ServiceError.NotFound(\"{kind}\", id.raw.toString)))", "      at <- now",
             f"      _ <- publish(Event.Removed(\"{kind}\", id.raw.toString, at))", "    yield ()", ""]
    for e in customs:
        params = [(n, t) for n, t in zip([n for n, _ in e.captures] + [n for n, _ in e.queries] + ["trace" for _ in e.headers] + (["body"] if e.body is not None else []), e.inputs())]
        sig = ", ".join(f"{safe(n)}: {t.expr}" for n, t in params)
        out_t = e.output_type()
        body = [f"  def {e.name}({sig}): Op[{out_t}] =", "    for", "      item <- get(id)", "      at <- now",
                f"      _ <- log.info(\"{e.name}\", \"id\" -> id.raw.toString)",
                f"      _ <- publish(Event.Custom(\"{kind}\", id.raw.toString, \"{e.name}\", at))"]
        if status and enum_decl and rng.chance(0.5):
            target = rng.pick(enum_decl.cases[1:]) if len(enum_decl.cases) > 1 else enum_decl.cases[0]
            body.append(f"      changed <- Eff.fromEither({area}Rules.transition(item, {enum_decl.name}.{target})).orElse(Eff.pure(item))")
            body.append("      r <- repo")
            body.append("      _ <- attempt(r.update(changed))")
            result = "changed" if out_t == en else "()"
        else:
            result = "item" if out_t == en else "()"
        body.append(f"    yield {result}")
        body.append("")
        lines += body
    Q = f"{area}Queries"
    A = f"{area}Audit"
    text = next((n for n, t in entity.fields if t.expr == "String"), None)
    lines += [f"  private val trail = new {area}Trail(limit = 200)",
              "",
              f"  def listing(listing: {Q}.Listing = {Q}.default): Op[List[{en}]] =", "    for", "      r <- repo",
              "      items <- attempt(r.query(listing))",
              f"      _ <- log.debug(\"listing \" + {Q}.describe(listing))", "    yield items", "",
              f"  def search(text: String, limit: Int = 20): Op[List[{en}]] =", "    for", "      r <- repo",
              f"      items <- attempt({'r.search' + cap(text) + '(text, limit)' if text else 'r.all(limit)'})",
              f"      _ <- log.info(\"search\", \"text\" -> text, \"hits\" -> items.length.toString)", "    yield items", "",
              f"  def change(id: {idn}, incoming: {en}, actor: String, note: Option[String] = None): Op[List[{A}.Change]] =", "    for",
              "      current <- get(id)", f"      merged = {area}Rules.merge(current, incoming)", f"      changes = {A}.changes(current, merged)",
              "      _ <- Eff.when(changes.nonEmpty)(update(id, merged))", "      at <- now",
              f"      _ <- Eff.when(changes.nonEmpty || note.isDefined)(trail.record({A}.entry(id, actor, changes, at, note = note)))",
              "    yield changes", "",
              f"  def archive(id: {idn}, actor: String, reason: Option[String] = None): Op[Unit] =", "    for", "      item <- get(id)", "      at <- now",
              f"      _ <- trail.record({A}.entry(id = item.id, actor = actor, changes = Nil, at = at, note = Some(reason.getOrElse(\"archived\"))))",
              f"      _ <- publish(Event.Custom(\"{kind}\", id.raw.toString, \"archive\", at))",
              f"      _ <- log.info(\"archive\", \"id\" -> id.raw.toString, \"actor\" -> actor)", "    yield ()", "",
              f"  def history(id: {idn}): Op[List[{A}.Entry]] = trail.forItem(id)",
              f"  def recentActors: Op[List[String]] = trail.all.map({A}.actors)",
              f"  def significantChanges(id: {idn}): Op[List[{A}.Entry]] = history(id).map(_.filter({A}.significant))", "",
              f"  def perform(action: String, id: {idn}, actor: String): Op[String] = action match",
              "    case \"touch\" => touch(id).map(_ => \"touched\")",
              "    case \"archive\" => archive(id, actor).as(\"archived\")",
              "    case \"describe\" => describe(id)",
              "    case \"remove\" => remove(id).as(\"removed\")",
              "    case \"history\" => history(id).map(entries => entries.length.toString + \" entries\")",
              "    case other => Eff.fail(ServiceError.Invalid(\"unknown action \" + other))", "",
              f"  def bulkCreate(items: List[{en}]): Op[Int] =", "    for", "      created <- Eff.foreach(items)(item => create(item).either)",
              "      failures = created.count(_.isLeft)", f"      _ <- Eff.when(failures > 0)(log.warn(failures.toString + \" of \" + items.length + \" failed\"))", "    yield items.length - failures", "",
              f"  def stats: Op[{area}Stats] =", "    for", "      r <- repo", "      items <- attempt(r.all(1000))", "      count <- attempt(r.count)",
              f"    yield {area}Reports.stats(items, count)", "",
              f"  def touch(id: {idn}): Op[{en}] = get(id).flatMap(item => update(id, item).as(item))",
              f"  def getOrCreate(item: {en}): Op[{en}] = get(item.id).catchAll {{",
              f"    case ServiceError.NotFound(_, _) => create(item)", "    case other => Eff.fail(other)", "  }",
              f"  def describe(id: {idn}): Op[String] = get(id).map({area}Rules.summary)"]
    ctx.write(f"business.{area.lower()}", f"{area}Service", "\n".join(lines) + "\n", extra="import meridian.business.*")


def write_export(ctx, area, decls, entity):
    en = entity.name
    fields = [(n, t) for n, t in entity.fields if n != "id"][:9]
    lines = [f"/** The {area.lower()} items as operators export them: columns, a text table, comma-separated lines. */",
             f"object {area}Export:",
             "  final case class Column(",
             "    header: String,",
             f"    render: {en} => String,",
             "    width: Int = 12,",
             "    rightAligned: Boolean = false",
             "  )",
             "",
             "  val columns: List[Column] = List(",
             "    Column(header = \"id\", render = _.id.raw.toString, width = 8),"]
    cols = []
    for n, t in fields:
        numeric = t.expr in ("Int", "Long", "Amount")
        width = 6 if t.expr in ("Boolean", "Int") else 10 if numeric else 20
        extra = ", rightAligned = true" if numeric else ""
        cols.append(f"    Column(header = \"{n}\", render = item => item.{n}.toString, width = {width}{extra})")
    lines.append(",\n".join(cols))
    lines += ["  )", "",
              f"  def cells(item: {en}): List[String] = columns.map(_.render(item))",
              f"  def table(items: List[{en}]): String = Text.column(columns.map(_.header) :: items.map(cells))",
              f"  def csv(items: List[{en}]): String =",
              "    (columns.map(_.header) :: items.map(cells)).map(_.map(escape).mkString(\",\")).mkString(\"\\n\")",
              "",
              "  def escape(text: String): String =",
              "    val cleaned = text.toList.foldLeft(\"\") { (acc, c) =>",
              "      c match",
              "        case '\"' => acc + \"\\\"\\\"\"",
              "        case ',' | '\\n' => acc + \" \"",
              "        case other => acc + other",
              "    }",
              "    if cleaned == text then text else \"\\\"\" + cleaned + \"\\\"\"",
              "",
              "  def pad(cell: String, column: Column): String =",
              "    val shown = cell.take(column.width)",
              "    val filler = \" \" * (column.width - shown.length)",
              "    if column.rightAligned then filler + shown else shown + filler",
              f"  def fixed(items: List[{en}]): String =",
              "    items.map(item => columns.zip(cells(item)).map((column, value) => pad(value, column)).mkString(\" \")).mkString(\"\\n\")",
              "",
              f"  def summary(items: List[{en}]): String =",
              f"    s\"\"\"{area.lower()}: ${{items.length}} rows",
              "       |columns: ${columns.map(_.header).mkString(\", \")}",
              "       |widest: ${columns.sortBy(-_.width).headOption.map(_.header).getOrElse(\"-\")}\"\"\".stripMargin"]
    ctx.write(f"business.{area.lower()}", f"{area}Export", "\n".join(lines) + "\n", extra="import meridian.business.*")


def write_notifications(ctx, area, decls, entity):
    kind = area.lower()
    lines = [f"/** Who hears about {kind} events, and the digest they get at the end of a night. */",
             f"object {area}Notifications:",
             "  def recipientFor(event: Event): Option[String] = event match",
             f"    case Event.Created(kind, _, _) if kind == \"{kind}\" => Some(\"{kind}-watchers@example\")",
             f"    case Event.Updated(kind, _, fields, _) if kind == \"{kind}\" && fields.nonEmpty => Some(\"{kind}-watchers@example\")",
             f"    case Event.Removed(kind, _, _) if kind == \"{kind}\" => Some(\"{kind}-audit@example\")",
             f"    case Event.Failed(kind, _, _) if kind == \"{kind}\" => Some(\"oncall@example\")",
             "    case _ => None",
             "",
             "  def onEvent(event: Event): Option[Message] = recipientFor(event).map(to => Mailer.compose(to, event))",
             "",
             "  def digest(events: List[Event]): String =",
             f"    val mine = events.filter(_.kind == \"{kind}\")",
             "    val created = mine.count { case Event.Created(_, _, _) => true; case _ => false }",
             "    val updated = mine.count { case Event.Updated(_, _, _, _) => true; case _ => false }",
             "    val removed = mine.count { case Event.Removed(_, _, _) => true; case _ => false }",
             f"    s\"\"\"{kind} digest",
             "       |created: $created",
             "       |updated: $updated",
             "       |removed: $removed",
             "       |other: ${mine.length - created - updated - removed}\"\"\".stripMargin",
             "",
             "  def batch(events: List[Event], mailer: Mailer): UIO[List[Delivery]] =",
             "    Eff.foreach(events.flatMap(onEvent))(mailer.send)",
             "",
             "  def urgency(event: Event): Int = event match",
             "    case Event.Failed(_, _, _) => 1",
             "    case Event.Removed(_, _, _) => 2",
             "    case Event.Updated(_, _, fields, _) if fields.length > 3 => 3",
             "    case _ => 5"]
    ctx.write(f"business.{area.lower()}", f"{area}Notifications", "\n".join(lines) + "\n", extra="import meridian.business.*\nimport meridian.infra.mail.*")


def safe(name):
    return name if name.isidentifier() and name not in ("type",) else name.replace("-", "").lower() + "Value"


def write_reports(ctx, area, decls, entity):
    en = entity.name
    status = next(((n, t) for n, t in entity.fields if t.kind == "enum"), None)
    enum_decl = next((d for d in decls if status and d.name == status[1].expr), None)
    numbers = [n for n, t in entity.fields if t.expr in ("Int", "Long")][:3]
    amounts = [n for n, t in entity.fields if t.expr == "Amount"][:1]
    dates = [n for n, t in entity.fields if t.expr in ("Instant", "LocalDate")][:1]
    bools = [n for n, t in entity.fields if t.expr == "Boolean"][:2]
    texts = [n for n, t in entity.fields if t.expr == "String"][:1]
    lines = [f"/** What the {area.lower()} reports say about a set of items. */",
             f"final case class {area}Stats(count: Int, stored: Int, byStatus: Map[String, Int], numbers: Map[String, Long], flags: Map[String, Int], newest: Option[String]) derives JsonCodec, Schema:",
             "  def render: String =",
             "    val statuses = byStatus.toList.sortBy(_._1).map((k, v) => s\"$k=$v\").mkString(\",\")",
             "    val sums = numbers.toList.sortBy(_._1).map((k, v) => s\"$k=$v\").mkString(\",\")",
             "    s\"count=$count stored=$stored statuses=[$statuses] sums=[$sums] flags=${flags.size} newest=${newest.getOrElse(\"-\")}\"",
             "",
             f"object {area}Reports:",
             f"  def stats(items: List[{en}], stored: Int): {area}Stats =",
             f"    {area}Stats(", "      items.length,", "      stored,"]
    lines.append(f"      items.groupBy(_.{status[0]}.toString).map((k, v) => (k, v.length))," if status else "      Map(\"all\" -> items.length),")
    sums = [f'"{n}" -> items.map(_.{n}.toLong).sum' for n in numbers]
    lines.append("      Map(" + ", ".join(sums) + ")," if sums else "      Map.empty,")
    flags = [f'"{n}" -> items.count(_.{n})' for n in bools]
    lines.append("      Map(" + ", ".join(flags) + ")," if flags else "      Map.empty,")
    lines.append(f"      items.sortBy(_.{dates[0]}).lastOption.map(_.id.raw.toString))" if dates else "      items.lastOption.map(_.id.raw.toString))")
    lines += ["", f"  def buckets(items: List[{en}]): List[(String, Int, Int)] ="]
    if status:
        lines.append(f"    items.groupBy(_.{status[0]}.toString).toList.sortBy(_._1).map((k, v) => (k, v.length, share(v.length, items.length)))")
    else:
        lines.append(f"    items.groupBy(i => i.id.raw.toString.take(1)).toList.sortBy(_._1).map((k, v) => (k, v.length, share(v.length, items.length)))")
    lines += ["  def share(part: Int, whole: Int): Int = if whole == 0 then 0 else part * 100 / whole",
              f"  def top(items: List[{en}], n: Int): List[{en}] = items.sortBy(i => -weight(i)).take(n)",
              f"  def weight(item: {en}): Long = " + (" + ".join(f"item.{n}.toLong" for n in numbers) if numbers else "item.id.raw.toString.length.toLong"),
              f"  def histogram(items: List[{en}], step: Long): Map[Long, Int] = items.groupBy(i => weight(i) / Math.max(step, 1L)).map((k, v) => (k, v.length))",
              f"  def render(items: List[{en}]): String =",
              "    val rows = buckets(items).map((label, count, share) => List(label, count.toString, share.toString + \"%\"))",
              "    Text.column(List(\"bucket\", \"count\", \"share\") :: rows)"]
    if amounts:
        lines += [f"  def spend(items: List[{en}]): Amount = items.map(_.{amounts[0]}).foldLeft[Amount](Amount.zero)(_ + _)",
                  f"  def average(items: List[{en}]): Amount = if items.isEmpty then Amount.zero else spend(items) / items.length.toLong"]
    if status and enum_decl:
        lines += [f"  def phase(items: List[{en}]): String = items.map(_.{status[0]}).distinct.length match",
                  "    case 0 => \"idle\"", "    case 1 => \"uniform\"", "    case n if n < 4 => \"mixed\"", "    case _ => \"scattered\"",
                  f"  def describe(value: {enum_decl.name}): String = value match"]
        for i, case in enumerate(enum_decl.cases[:6]):
            lines.append(f"    case {enum_decl.name}.{case} => \"{case.lower()}\"")
        if len(enum_decl.cases) > 6:
            lines.append("    case other => other.toString")
    if texts:
        lines += [f"  def wordFrequency(items: List[{en}]): Map[String, Int] =",
                  f"    items.flatMap(i => Text.words(i.{texts[0]})).groupBy(identity).map((k, v) => (k, v.length))",
                  f"  def commonWords(items: List[{en}], n: Int): List[String] = wordFrequency(items).toList.sortBy((k, v) => (-v, k)).take(n).map(_._1)"]
    lines += [f"  def digest(items: List[{en}]): Long = items.map(i => Checksum.of(i.toString)).foldLeft(17L)((a, b) => a * 31L + b)",
              f"  def sample(items: List[{en}], every: Int): List[{en}] = items.zipWithIndex.collect {{ case (i, n) if n % Math.max(every, 1) == 0 => i }}"]
    ctx.write(f"business.{area.lower()}", f"{area}Reports", "\n".join(lines) + "\n", extra="import meridian.business.*")


# ---------------------------------------------------------------------------------------------
# allocation, public

def write_allocation(ctx):
    ctx.write("allocation", "Budget", '''/** Observing-time budgets: what a proposal may spend on a telescope in a period. */
final case class Budget(granted: Amount, used: Amount, reserved: Amount):
  def remaining: Amount = granted - used - reserved
  def exhausted: Boolean = remaining.isNegative || remaining.isZero
  def usedShare: Int = if granted.isZero then 0 else (used.minor * 100L / granted.minor).toInt
  def spend(amount: Amount): Either[String, Budget] =
    if amount.isNegative then Left("negative spend")
    else if (remaining - amount).isNegative then Left(s"over budget by ${(amount - remaining).render}")
    else Right(copy(used = used + amount))
  def reserve(amount: Amount): Budget = copy(reserved = reserved + amount)
  def release(amount: Amount): Budget = copy(reserved = (reserved - amount).atLeast(Amount.zero))

object Budget:
  val empty: Budget = Budget(Amount.zero, Amount.zero, Amount.zero)
  def of(granted: Amount): Budget = Budget(granted, Amount.zero, Amount.zero)
  given cats.Monoid[Budget] = new cats.Monoid[Budget]:
    def empty = Budget.empty
    def combine(a: Budget, b: Budget) = Budget(a.granted + b.granted, a.used + b.used, a.reserved + b.reserved)
''')
    ctx.write("allocation", "Tariff", '''/** What an hour costs, by instrument class and by period. */
enum InstrumentClass(val label: String) derives Enumerated, Eq:
  case Imager extends InstrumentClass("imager")
  case Spectrograph extends InstrumentClass("spectrograph")
  case Polarimeter extends InstrumentClass("polarimeter")
  case Guider extends InstrumentClass("guider")

enum Period derives Enumerated, Eq:
  case Dark, Grey, Bright

final case class Tariff(rates: Map[(InstrumentClass, Period), Amount], surcharge: Percent):
  def rate(instrument: InstrumentClass, period: Period): Amount = rates.getOrElse((instrument, period), Amount.ofUnits(10L))
  def cost(instrument: InstrumentClass, period: Period, minutes: Long): Amount =
    val base = rate(instrument, period) * minutes / 60L
    base + Amount.ofMinor(surcharge.of(base.minor))

object Tariff:
  val standard: Tariff = Tariff(
    (for
      i <- Enumerated[InstrumentClass].valueList
      p <- Enumerated[Period].valueList
    yield ((i, p), Amount.ofUnits((Enumerated[InstrumentClass].ordinalOf(i) + 1).toLong * (3L - Enumerated[Period].ordinalOf(p).toLong)))).toMap,
    Percent.unsafeFrom(5))
''')
    ctx.write("allocation", "Plan", '''/** A night's plan: slots handed to proposals under their budgets. */
final case class Slot(proposal: String, instrument: InstrumentClass, period: Period, minutes: Long):
  def cost(tariff: Tariff): Amount = tariff.cost(instrument, period, minutes)

final case class Plan(night: LocalDate, slots: List[Slot]):
  def minutes: Long = slots.map(_.minutes).sum
  def byProposal: Map[String, List[Slot]] = slots.groupBy(_.proposal)
  def cost(tariff: Tariff): Amount = slots.map(_.cost(tariff)).foldLeft[Amount](Amount.zero)(_ + _)

enum PlanError derives Eq, Show:
  case OverBudget(proposal: String, by: Amount)
  case Overlap(minutes: Long)
  case Empty

object Planner:
  val nightMinutes: Long = 9L * 60L

  def check(plan: Plan, budgets: Map[String, Budget], tariff: Tariff): Check[Plan] =
    val length = Check.cond(plan.minutes <= nightMinutes, (), s"plan exceeds the night by ${plan.minutes - nightMinutes} minutes")
    val funded = Check.all(plan.byProposal.toList.map { (proposal, slots) =>
      val cost = slots.map(_.cost(tariff)).foldLeft[Amount](Amount.zero)(_ + _)
      val budget = budgets.getOrElse(proposal, Budget.empty)
      Check.cond(!(budget.remaining - cost).isNegative, (), s"$proposal over budget by ${(cost - budget.remaining).render}")
    })
    length.zip(funded).as(plan)

  def charge(plan: Plan, budgets: Map[String, Budget], tariff: Tariff): Either[PlanError, Map[String, Budget]] =
    if plan.slots.isEmpty then Left(PlanError.Empty)
    else if plan.minutes > nightMinutes then Left(PlanError.Overlap(plan.minutes - nightMinutes))
    else
      plan.byProposal.toList.foldLeft[Either[PlanError, Map[String, Budget]]](Right(budgets)) { case (acc, (proposal, slots)) =>
        acc.flatMap { current =>
          val cost = slots.map(_.cost(tariff)).foldLeft[Amount](Amount.zero)(_ + _)
          current.getOrElse(proposal, Budget.empty).spend(cost) match
            case Right(b) => Right(current.updated(proposal, b))
            case Left(_) => Left(PlanError.OverBudget(proposal, cost - current.getOrElse(proposal, Budget.empty).remaining))
        }
      }

  def fill(requests: List[(String, InstrumentClass, Long)], period: Period, night: LocalDate): Plan =
    val slots = requests.foldLeft((List.empty[Slot], 0L)) { case ((acc, used), (proposal, instrument, minutes)) =>
      val granted = Math.min(minutes, nightMinutes - used)
      if granted <= 0 then (acc, used) else (acc :+ Slot(proposal, instrument, period, granted), used + granted)
    }
    Plan(night, slots._1)
''')
    ctx.write("allocation", "Quota", '''/** Per-semester quotas with rounding to the tariff's granularity. */
final case class Quota(proposal: String, hours: Int, priority: Int):
  def minutes: Long = hours.toLong * 60L
  def weight: Long = priority.toLong * minutes

object Quota:
  def share(quotas: List[Quota], available: Long): Map[String, Long] =
    val total = quotas.map(_.weight).sum
    if total == 0 then Map.empty
    else quotas.map(q => (q.proposal, roundTo(q.weight * available / total, 15L))).toMap
  def roundTo(minutes: Long, step: Long): Long = (minutes + step / 2) / step * step
  def rank(quotas: List[Quota]): List[Quota] = quotas.sortBy(q => (-q.priority, -q.hours, q.proposal))
  def summary(quotas: List[Quota]): String = quotas.length match
    case 0 => "no quotas"
    case 1 => s"one quota of ${quotas.head.hours}h"
    case n => s"$n quotas, ${quotas.map(_.hours).sum}h"
''')
    ctx.write("allocation", "Calendar", '''/** Nights of a semester and which period each falls in. */
object Calendar:
  def nights(from: LocalDate, count: Int): List[LocalDate] = (0 until count).toList.map(i => from.plusDays(i.toLong))
  def period(night: LocalDate): Period =
    (night.toEpochDay % 29L) match
      case n if n < 8 => Period.Dark
      case n if n < 18 => Period.Grey
      case _ => Period.Bright
  def darkNights(from: LocalDate, count: Int): List[LocalDate] = nights(from, count).filter(n => period(n) == Period.Dark)
  def weekends(from: LocalDate, count: Int): Int = nights(from, count).count(_.getDayOfWeek.isWeekend)
  def label(night: LocalDate): String = s"${night.toString} ${period(night).entryName.toLowerCase}"
''')
    ctx.write("allocation", "Summary", '''/** A semester's allocation summed up for a report. */
final case class AllocationSummary(nights: Int, dark: Int, planned: Long, charged: Amount, proposals: Int) derives JsonCodec, Schema:
  def render: String = s"nights=$nights dark=$dark planned=$planned charged=${charged.render} proposals=$proposals"

object Summary:
  def of(from: LocalDate, count: Int, quotas: List[Quota], tariff: Tariff): AllocationSummary =
    val nights = Calendar.nights(from, count)
    val shares = Quota.share(quotas, count.toLong * Planner.nightMinutes)
    val plans = nights.map(n => Planner.fill(quotas.map(q => (q.proposal, InstrumentClass.Imager, shares.getOrElse(q.proposal, 0L) / Math.max(count, 1))), Calendar.period(n), n))
    val charged = plans.map(_.cost(tariff)).foldLeft[Amount](Amount.zero)(_ + _)
    AllocationSummary(nights.length, Calendar.darkNights(from, count).length, plans.map(_.minutes).sum, charged, quotas.length)
''')


def write_public(ctx):
    rng = ctx.rng
    areas = ctx.areas[:6]
    lines = ["/** What the public site may read: a slice of the model with its own schemas. */"]
    dtos = []
    for area, decls, entities in areas:
        entity = entities[0] if entities else None
        if entity is None:
            continue
        fields = [(n, t) for n, t in entity.fields if n != "id" and t.kind == "prim" and t.text][:4]
        name = f"{entity.name}Published"
        lines.append(f"final case class {name}(" + ", ".join(["id: String"] + [f"{n}: {t.expr}" for n, t in fields]) + ") derives JsonCodec, Schema")
        lines.append(f"object {name}:")
        lines.append(f"  def from(item: {entity.name}): {name} = {name}(item.id.raw.toString{''.join(f', item.{n}' for n, _ in fields)})")
        lines.append("")
        dtos.append((area, entity, name))
    ctx.write("public", "PublicModels", "\n".join(lines) + "\n")
    routes = ["/** The public endpoints, unauthenticated and read-only. */", "object PublicRoutes:",
              '  val publicEndpoint: Endpoint[Unit, Unit, ApiFailure, Unit] = endpoint.errorOut(jsonOut[ApiFailure])']
    for area, entity, name in dtos:
        routes.append(f'  val list{entity.name} = publicEndpoint.get.in("public" / "{area.lower()}").in(query[Option[Int]]("limit")).out(jsonOut[List[{name}]])')
        routes.append(f'  val get{entity.name} = publicEndpoint.get.in("public" / "{area.lower()}" / path[String]("id")).out(jsonOut[{name}])')
    routes.append('  val status = publicEndpoint.get.in("public" / "status").out(jsonOut[PublicStatus])')
    routes.append("")
    routes.append("final case class PublicStatus(version: String, sites: List[String], healthy: Boolean, nights: Int) derives JsonCodec, Schema")
    ctx.write("public", "PublicRoutes", "\n".join(routes) + "\n")
    for i, (area, entity, name) in enumerate(dtos[:8]):
        ctx.write("public", f"{name}Schema", f'''/** The documented shape of {name}, with its examples. */
object {name}Schema:
  val schema: Schema[{name}] = Schema[{name}].named("{name}").describe("public view of {area.lower()}")
  val fields: Int = schema.fieldCount
  val example: {name} = {name}.from({area}Samples.{entity.sample_name()})
  def render: String = Schema.render(schema.tpe)
''', extra="import meridian.model.samples.*")


# ---------------------------------------------------------------------------------------------
# server

def write_server(ctx):
    ctx.write("business", "ServiceError", '''/** What a service may fail with, and how the routes answer it. */
enum ServiceError derives Eq, Show:
  case NotFound(kind: String, id: String)
  case Invalid(message: String)
  case Conflict(message: String)
  case Forbidden(reason: String)
  case Failure(message: String)
  def toApi: ApiFailure = this match
    case NotFound(kind, id) => ApiFailure.NotFound(s"$kind $id")
    case Invalid(message) => ApiFailure.Invalid(message)
    case Conflict(message) => ApiFailure.Conflict(message)
    case Forbidden(reason) => ApiFailure.Forbidden(reason)
    case Failure(message) => ApiFailure.Invalid(s"failure: $message")
''')
    for area, decls, entities in ctx.areas:
        write_routes(ctx, area, decls, entities)
    write_executor(ctx)
    write_platform(ctx)
    write_wiring(ctx)
    write_views(ctx)
    write_main(ctx)


def write_platform(ctx):
    ctx.write("server.config", "Settings", r"""/** What the server is configured with, read from an environment map with defaults for everything. */
enum Profile derives Enumerated:
  case Local, Staging, Production

final case class HttpSettings(
  host: String = "127.0.0.1",
  port: Int = 8080,
  basePath: String = "/v1",
  maxBodyBytes: Int = 1 << 20,
  requestTimeout: Duration = Duration.ofSeconds(30L),
  corsOrigins: List[String] = Nil
)

final case class DatabaseSettings(
  url: String = "mem://meridian",
  user: String = "meridian",
  poolSize: Int = 8,
  statementTimeout: Duration = Duration.ofSeconds(10L),
  migrate: Boolean = true
)

final case class AuthSettings(
  tokenHeader: String = "X-Token",
  siteHeader: String = "X-Site-Key",
  tokenTtl: Duration = Duration.ofSeconds(3600L),
  anonymousReads: Boolean = false,
  admins: List[String] = List("operator-1")
)

final case class FeedSettings(
  enabled: List[String] = Nil,
  pollEvery: Duration = Duration.ofSeconds(300L),
  retries: Int = 2,
  staleAfter: Duration = Duration.ofSeconds(1800L)
)

final case class Settings(
  profile: Profile = Profile.Local,
  http: HttpSettings = HttpSettings(),
  database: DatabaseSettings = DatabaseSettings(),
  auth: AuthSettings = AuthSettings(),
  feeds: FeedSettings = FeedSettings(),
  features: Map[String, Boolean] = Map.empty
):
  def isProduction: Boolean = profile == Profile.Production
  def feature(name: String): Boolean = features.getOrElse(name, false)
  def render: String =
    List(
      "profile=" + profile.entryName,
      "http=" + http.host + ":" + http.port + http.basePath,
      "db=" + database.url + " pool=" + database.poolSize,
      "auth=" + auth.tokenHeader + " ttl=" + auth.tokenTtl.toSeconds + "s",
      "feeds=" + feeds.enabled.mkString(",") + " every=" + feeds.pollEvery.toSeconds + "s",
      "features=" + features.toList.sortBy(_._1).map((k, v) => k + (if v then "+" else "-")).mkString(",")
    ).mkString(" ")

object Settings:
  def fromEnv(env: Map[String, String]): Either[String, Settings] =
    for
      profile <- env.get("PROFILE").fold[Either[String, Profile]](Right(Profile.Local))(parseProfile)
      port <- number(env, "HTTP_PORT", 8080)
      pool <- number(env, "DB_POOL", 8)
      retries <- number(env, "FEED_RETRIES", 2)
    yield Settings(
      profile = profile,
      http = HttpSettings(host = env.getOrElse("HTTP_HOST", "127.0.0.1"), port = port, corsOrigins = list(env, "CORS_ORIGINS")),
      database = DatabaseSettings(url = env.getOrElse("DB_URL", "mem://meridian"), poolSize = pool, migrate = flag(env, "DB_MIGRATE", true)),
      auth = AuthSettings(anonymousReads = flag(env, "AUTH_ANONYMOUS_READS", false), admins = list(env, "AUTH_ADMINS")),
      feeds = FeedSettings(enabled = list(env, "FEEDS"), retries = retries),
      features = list(env, "FEATURES").map(name => (name, true)).toMap
    )

  def parseProfile(name: String): Either[String, Profile] = name.toLowerCase match
    case "local" | "dev" => Right(Profile.Local)
    case "staging" | "test" => Right(Profile.Staging)
    case "production" | "prod" => Right(Profile.Production)
    case other => Left("unknown profile " + other)

  private def number(env: Map[String, String], key: String, default: Int): Either[String, Int] =
    env.get(key) match
      case None => Right(default)
      case Some(text) => text.toIntOption.toRight(key + " is not a number: " + text)

  private def flag(env: Map[String, String], key: String, default: Boolean): Boolean =
    env.get(key).map(_.toLowerCase) match
      case Some("true") | Some("1") | Some("yes") => true
      case Some("false") | Some("0") | Some("no") => false
      case _ => default

  private def list(env: Map[String, String], key: String): List[String] =
    val (items, last) = env.getOrElse(key, "").toList.foldLeft((List.empty[String], "")) { case ((done, current), c) =>
      if c == ',' then (current.trim :: done, "") else (done, current + c)
    }
    (last.trim :: items).reverse.filter(_.nonEmpty)
""")
    ctx.write("server.config", "Features", r"""/** Feature switches: what the server does differently per site or profile. */
enum Feature derives Enumerated, Eq:
  case AuditTrail, BulkImport, PublicReports, FeedSync, StrictValidation, Metrics, RateLimits, Experimental

final class Features(enabled: Set[Feature], overrides: Map[String, Set[Feature]] = Map.empty):
  def has(feature: Feature): Boolean = enabled.contains(feature)
  def forSite(site: Option[String]): Set[Feature] = site.flatMap(overrides.get).getOrElse(enabled)
  def describe(feature: Feature): String = feature match
    case Feature.AuditTrail => "changes are recorded with their actor"
    case Feature.BulkImport => "many items in one request"
    case Feature.PublicReports => "reports without credentials"
    case Feature.FeedSync => "providers polled on a schedule"
    case Feature.StrictValidation => "every rule fails the request"
    case Feature.Metrics => "timings kept per route"
    case Feature.RateLimits => "requests counted per token"
    case Feature.Experimental => "everything not yet named"
  def render: String = Enumerated[Feature].valueList.map(f => f.entryName + (if has(f) then "=on" else "=off")).mkString(" ")
  def toggled(feature: Feature): Features = new Features(if has(feature) then enabled - feature else enabled + feature, overrides)

object Features:
  val none: Features = new Features(Set.empty)
  val all: Features = new Features(Enumerated[Feature].valueList.toSet)
  def fromSettings(settings: Settings): Features =
    new Features(Enumerated[Feature].valueList.filter(f => settings.feature(f.entryName)).toSet)
  def standard(profile: Profile): Features = profile match
    case Profile.Local => all
    case Profile.Staging => new Features(Set(Feature.AuditTrail, Feature.FeedSync, Feature.Metrics, Feature.StrictValidation))
    case Profile.Production => new Features(Set(Feature.AuditTrail, Feature.FeedSync, Feature.Metrics, Feature.RateLimits))
""")
    ctx.write("server.auth", "Principal", r"""/** Who is calling: the subject behind a token, its roles and where it may act. */
enum Role derives Enumerated, Eq:
  case Viewer, Operator, Scheduler, Admin

final case class Principal(
  subject: String,
  roles: Set[Role],
  site: Option[String] = None,
  expires: Option[Instant] = None
):
  def has(role: Role): Boolean = roles.contains(role) || roles.contains(Role.Admin)
  def isExpired(now: Instant): Boolean = expires.exists(_.isBefore(now))
  def render: String = subject + roles.toList.map(_.entryName).sorted.mkString("[", ",", "]") + site.fold("")("@" + _)

enum AuthError derives Eq:
  case MissingToken
  case UnknownToken(token: String)
  case Expired(subject: String)
  case Forbidden(subject: String, needed: Permission)
  def message: String = this match
    case MissingToken => "no token"
    case UnknownToken(token) => "unknown token " + token.take(4)
    case Expired(subject) => subject + " expired"
    case Forbidden(subject, needed) => subject + " lacks " + needed.entryName

trait Authenticator:
  def authenticate(credentials: Credentials): IO[AuthError, Principal]
  def describe: String

final class TokenAuthenticator(tokens: Map[String, Principal], clock: Clock) extends Authenticator:
  def authenticate(credentials: Credentials): IO[AuthError, Principal] =
    for
      token <- Eff.cond(credentials.token.nonEmpty, credentials.token, AuthError.MissingToken)
      principal <- Eff.fromOption(tokens.get(token)).mapError(_ => AuthError.UnknownToken(token))
      _ <- Eff.cond(!principal.isExpired(clock.now), (), AuthError.Expired(principal.subject))
    yield principal.copy(site = credentials.siteKey.orElse(principal.site))
  def describe: String = tokens.size.toString + " tokens"

object Authenticator:
  val anonymous: Authenticator = new Authenticator:
    def authenticate(credentials: Credentials): IO[AuthError, Principal] =
      Eff.pure(Principal("anonymous", Set(Role.Viewer), credentials.siteKey))
    def describe: String = "anonymous"
  def fixed(clock: Clock, settings: AuthSettings): Authenticator =
    val admins = settings.admins.map(name => (name, Principal(name, Set(Role.Admin))))
    val operators = List(
      ("token-1", Principal("operator-1", Set(Role.Operator, Role.Scheduler), Some("north"))),
      ("token-2", Principal("viewer-2", Set(Role.Viewer))),
      ("token-3", Principal("retired-3", Set(Role.Operator), None, Some(clock.now.minus(Duration.ofHours(1L)))))
    )
    new TokenAuthenticator((admins ++ operators).toMap, clock)
""", extra="import meridian.server.config.*")
    ctx.write("server.auth", "Policy", r"""/** What each route needs, and whether a principal has it. */
enum Permission derives Enumerated, Eq:
  case Read, Write, Remove, Schedule, Administer

object Policy:
  def required(method: Method, segments: List[String]): Permission = (method, segments) match
    case (Method.Get, _) => Permission.Read
    case (Method.Delete, _) => Permission.Remove
    case (_, "v1" :: "admin" :: _) => Permission.Administer
    case (_, "v1" :: _ :: _ :: "schedule" :: _) => Permission.Schedule
    case (Method.Post, _) | (Method.Put, _) => Permission.Write
  def granted(role: Role): Set[Permission] = role match
    case Role.Viewer => Set(Permission.Read)
    case Role.Operator => Set(Permission.Read, Permission.Write, Permission.Remove)
    case Role.Scheduler => Set(Permission.Read, Permission.Schedule)
    case Role.Admin => Enumerated[Permission].valueList.toSet
  def allows(principal: Principal, permission: Permission): Boolean =
    principal.roles.exists(role => granted(role).contains(permission))
  def check(principal: Principal, permission: Permission): Either[AuthError, Principal] =
    if allows(principal, permission) then Right(principal) else Left(AuthError.Forbidden(principal.subject, permission))
  def explain(principal: Principal): String =
    Enumerated[Permission].valueList.map(p => p.entryName + (if allows(principal, p) then "+" else "-")).mkString(" ")

final class Guard(authenticator: Authenticator, features: Features):
  def admit(credentials: Credentials, method: Method, segments: List[String]): IO[AuthError, Principal] =
    for
      principal <- authenticator.authenticate(credentials)
      needed = Policy.required(method, segments)
      _ <- Eff.fromEither(Policy.check(principal, needed))
      _ <- Eff.when(needed == Permission.Write && features.has(Feature.StrictValidation))(Eff.unit)
    yield principal
  def admitAll(credentials: Credentials, requests: List[(Method, List[String])]): IO[AuthError, List[Principal]] =
    Eff.foreach(requests)((method, segments) => admit(credentials, method, segments))
  def describe: String = authenticator.describe + ", " + features.render
""", extra="import meridian.server.config.*")
    ctx.write("server.http", "Middleware", r"""/** Stages around a handler, over any effect a Sequencer knows how to sequence. */
trait Sequencer[F[_]]:
  def pure[A](a: A): F[A]
  def flatMap[A, B](fa: F[A])(f: A => F[B]): F[B]
  def map[A, B](fa: F[A])(f: A => B): F[B] = flatMap(fa)(a => pure(f(a)))

type Attempt[+A] = Either[String, A]

object Sequencer:
  given task: Sequencer[Task] with
    def pure[A](a: A): Task[A] = Eff.pure(a)
    def flatMap[A, B](fa: Task[A])(f: A => Task[B]): Task[B] = fa.flatMap(f)
  given attempt: Sequencer[Attempt] with
    def pure[A](a: A): Attempt[A] = Right(a)
    def flatMap[A, B](fa: Attempt[A])(f: A => Attempt[B]): Attempt[B] = fa.flatMap(f)

trait Middleware[F[_]]:
  def apply(request: Request, next: Request => F[Response]): F[Response]
  def andThen(that: Middleware[F]): Middleware[F] = Middleware.chain(List(this, that))

final class Headers[F[_]](extra: List[(String, String)])(using S: Sequencer[F]) extends Middleware[F]:
  def apply(request: Request, next: Request => F[Response]): F[Response] =
    S.map(next(request))(response => response.copy(headers = response.headers ++ extra))

final class Trace[F[_]](header: String)(using S: Sequencer[F]) extends Middleware[F]:
  def apply(request: Request, next: Request => F[Response]): F[Response] =
    val id = request.headers.collectFirst { case (k, v) if k == header => v }.getOrElse("trace-" + request.segments.length)
    val traced = request.copy(headers = (header, id) :: request.headers.filterNot(_._1 == header))
    S.map(next(traced))(response => response.copy(headers = (header, id) :: response.headers))

final class Timing[F[_]](clock: Clock, record: (Request, Long) => Unit)(using S: Sequencer[F]) extends Middleware[F]:
  def apply(request: Request, next: Request => F[Response]): F[Response] =
    val started = clock.now
    S.map(next(request)) { response =>
      record(request, clock.now.toEpochMilli - started.toEpochMilli)
      response
    }

final class Reject[F[_]](maxSegments: Int)(using S: Sequencer[F]) extends Middleware[F]:
  def apply(request: Request, next: Request => F[Response]): F[Response] =
    if request.segments.length > maxSegments then S.pure(Response(414, Nil, Some(quote("path too long")))) else next(request)

final class Fallback[F[_]](status: Int)(using S: Sequencer[F]) extends Middleware[F]:
  def apply(request: Request, next: Request => F[Response]): F[Response] =
    S.map(next(request))(response => if response.status == 404 && request.segments.isEmpty then response.copy(status = status) else response)

object Middleware:
  def chain[F[_]](stages: List[Middleware[F]]): Middleware[F] = new Middleware[F]:
    def apply(request: Request, next: Request => F[Response]): F[Response] =
      stages.foldRight(next)((stage, rest) => r => stage(r, rest))(request)
  def identity[F[_]]: Middleware[F] = new Middleware[F]:
    def apply(request: Request, next: Request => F[Response]): F[Response] = next(request)
  def standard[F[_]](clock: Clock, record: (Request, Long) => Unit)(using Sequencer[F]): Middleware[F] =
    chain(List(Trace("X-Trace"), Timing(clock, record), Reject(12), Headers(List(("X-Server", "meridian"))), Fallback(200)))
""")
    ctx.write("server.http", "StatusCodes", r"""/** Status codes: from failures to numbers and back to phrases. */
object StatusCodes:
  def of(failure: ApiFailure): Int = failure match
    case ApiFailure.NotFound(_) => 404
    case ApiFailure.Invalid(_) => 400
    case ApiFailure.Conflict(_) => 409
    case ApiFailure.Forbidden(_) => 403
  def phrase(code: Int): String = code match
    case 200 => "ok"
    case 201 => "created"
    case 204 => "no content"
    case 400 => "bad request"
    case 401 => "unauthorised"
    case 403 => "forbidden"
    case 404 => "not found"
    case 409 => "conflict"
    case 414 => "uri too long"
    case 422 => "unprocessable"
    case 429 => "too many requests"
    case 500 => "server error"
    case other if other < 400 => "success"
    case _ => "error"
  def isSuccess(code: Int): Boolean = code >= 200 && code < 300
  def isClientError(code: Int): Boolean = code >= 400 && code < 500
  def isRetryable(code: Int): Boolean = code match
    case 429 | 502 | 503 | 504 => true
    case _ => false
  def family(code: Int): String = code / 100 match
    case 2 => "2xx"
    case 3 => "3xx"
    case 4 => "4xx"
    case 5 => "5xx"
    case _ => "other"
  def summarise(codes: List[Int]): Map[String, Int] = codes.groupBy(family).map((k, v) => (k, v.length))
""")
    ctx.write("server.http", "package", r"""/** Small request and response helpers shared by the handlers, the middleware and the self-test. */
def ok(body: String): Response = Response(200, Nil, Some(body))
def created(body: String): Response = Response(201, Nil, Some(body))
def noContent: Response = Response(204, Nil, None)
def badRequest(message: String): Response = Response(400, Nil, Some(quote(message)))
def unauthorised: Response = Response(401, Nil, Some(quote("unauthorised")))
def forbidden(reason: String): Response = Response(403, Nil, Some(quote(reason)))
def notFound(what: String): Response = Response(404, Nil, Some(quote("not found: " + what)))
def conflict(message: String): Response = Response(409, Nil, Some(quote(message)))
def serverError(message: String): Response = Response(500, Nil, Some(quote(message)))
def quote(text: String): String = "\"" + text + "\""
def jsonOf[A](value: A)(using encoder: JsonEncoder[A]): Response = ok(value.toJson)
def header(request: Request, name: String): Option[String] = request.headers.collectFirst { case (k, v) if k.equalsIgnoreCase(name) => v }
def withHeader(response: Response, name: String, value: String): Response = response.copy(headers = (name, value) :: response.headers)
def query(request: Request, name: String): Option[String] = request.query.collectFirst { case (k, v) if k == name => v }
def queryInt(request: Request, name: String): Option[Int] = query(request, name).flatMap(_.toIntOption)
def pathOf(request: Request): String = request.segments.mkString("/", "/", "")
def isJson(request: Request): Boolean = header(request, "Content-Type").exists(_.contains("json"))
def isRead(request: Request): Boolean = request.method == Method.Get
def describeRequest(request: Request): String = request.method.toString + " " + pathOf(request)
def describeResponse(response: Response): String = response.status.toString + " " + StatusCodes.phrase(response.status)
def sizeOf(response: Response): Int = response.body.fold(0)(_.length)
def statusesOf(responses: List[Response]): Map[String, Int] = StatusCodes.summarise(responses.map(_.status))
def under(request: Request, prefix: String): Boolean = request.segments.headOption.contains(prefix)
def rest(request: Request, prefix: String): List[String] = if under(request, prefix) then request.segments.drop(1) else request.segments
def traceId(request: Request): String = header(request, "X-Trace").getOrElse("-")
def renderHeaders(headers: List[(String, String)]): String = headers.map((k, v) => k + ": " + v).mkString("; ")
def redact(headers: List[(String, String)]): List[(String, String)] = headers.map((k, v) => if k.equalsIgnoreCase("X-Token") then (k, "***") else (k, v))
def merge(a: Response, b: Response): Response = if StatusCodes.isSuccess(a.status) then a else b
""")
    ctx.write("server.metrics", "Metrics", r"""/** Counters and timers per route, and where their lines go. */
final case class Counter(name: String, value: Long = 0L):
  def increment(by: Long = 1L): Counter = copy(value = value + by)

final case class Timer(name: String, count: Long = 0L, totalMillis: Long = 0L, maxMillis: Long = 0L):
  def observe(millis: Long): Timer = Timer(name, count + 1, totalMillis + millis, Math.max(maxMillis, millis))
  def mean: Long = if count == 0 then 0L else totalMillis / count

trait Sink:
  def emit(line: String): UIO[Unit]

object Sink:
  val silent: Sink = new Sink:
    def emit(line: String): UIO[Unit] = Eff.unit
  def collecting(ref: Ref[List[String]]): Sink = new Sink:
    def emit(line: String): UIO[Unit] = ref.update(line :: _)

final class Registry(sink: Sink):
  private var counters: Map[String, Counter] = Map.empty
  private var timers: Map[String, Timer] = Map.empty
  def count(name: String, by: Long = 1L): Unit = counters = counters.updated(name, counters.getOrElse(name, Counter(name)).increment(by))
  def time(name: String, millis: Long): Unit = timers = timers.updated(name, timers.getOrElse(name, Timer(name)).observe(millis))
  def counter(name: String): Long = counters.get(name).map(_.value).getOrElse(0L)
  def timer(name: String): Option[Timer] = timers.get(name)
  def flush: UIO[Int] = Eff.foreach(render)(sink.emit).map(_.length)
  def render: List[String] =
    counters.values.toList.sortBy(_.name).map(c => c.name + "=" + c.value) ++
      timers.values.toList.sortBy(_.name).map(t => t.name + " n=" + t.count + " mean=" + t.mean + "ms max=" + t.maxMillis + "ms")
  def reset(): Unit =
    counters = Map.empty
    timers = Map.empty

object Metrics:
  def routeName(request: Request): String =
    request.method.toString.toLowerCase + ":" + request.segments.map(s => if s.nonEmpty && s.forall(_.isDigit) then ":id" else s).mkString("/")
  def observe(registry: Registry)(request: Request, millis: Long): Unit =
    registry.count("requests")
    registry.count("requests." + request.method.toString.toLowerCase)
    registry.time(routeName(request), millis)
  def summary(registry: Registry): String =
    "requests=" + registry.counter("requests") + " routes=" + registry.render.count(_.contains(" n="))
""")
    ctx.write("server.health", "Health", r"""/** Probes over the services, and the verdict they add up to. */
enum HealthStatus derives Enumerated, Eq:
  case Up, Degraded, Down

final case class HealthCheck(name: String, status: HealthStatus, detail: Option[String] = None, millis: Long = 0L):
  def render: String = name + "=" + status.entryName + detail.fold("")(d => "(" + d + ")")

trait Probe:
  def name: String
  def run: UIO[HealthCheck]

object Probes:
  def database(xa: Transactor): Probe = new Probe:
    def name = "database"
    def run: UIO[HealthCheck] =
      Eff.succeed(HealthCheck(name, if xa.seen.length < 10000 then HealthStatus.Up else HealthStatus.Degraded, Some(xa.seen.length.toString + " statements")))
  def bus(bus: Bus): Probe = new Probe:
    def name = "bus"
    def run: UIO[HealthCheck] = Eff.succeed(HealthCheck(name, HealthStatus.Up, Some(bus.count.toString + " events")))
  def logger(logger: Logger): Probe = new Probe:
    def name = "logger"
    def run: UIO[HealthCheck] =
      Eff.succeed(HealthCheck(name, if logger.countBy(Level.Error) == 0 then HealthStatus.Up else HealthStatus.Degraded, Some(logger.count.toString + " lines")))
  def always(probeName: String, status: HealthStatus): Probe = new Probe:
    def name = probeName
    def run: UIO[HealthCheck] = Eff.succeed(HealthCheck(name, status))

object Health:
  def all(probes: List[Probe]): UIO[List[HealthCheck]] =
    for
      checks <- Eff.foreach(probes)(_.run)
    yield checks.sortBy(_.name)
  def overall(checks: List[HealthCheck]): HealthStatus = checks.map(_.status) match
    case Nil => HealthStatus.Down
    case statuses if statuses.contains(HealthStatus.Down) => HealthStatus.Down
    case statuses if statuses.contains(HealthStatus.Degraded) => HealthStatus.Degraded
    case _ => HealthStatus.Up
  def statusCode(status: HealthStatus): Int = status match
    case HealthStatus.Up => 200
    case HealthStatus.Degraded => 200
    case HealthStatus.Down => 503
  def render(checks: List[HealthCheck]): String = overall(checks).entryName + " " + checks.map(_.render).mkString(" ")
""")
    ctx.write("server.jobs", "Retry", r"""/** Retrying an operation a bounded number of times, with the reasons kept. */
final case class RetryPolicy(attempts: Int = 3, delay: Duration = Duration.ofSeconds(1L), backoff: Int = 2):
  def next: RetryPolicy = copy(attempts = attempts - 1, delay = Duration.ofMillis(delay.toMillis * backoff))
  def exhausted: Boolean = attempts <= 0

object Retry:
  def apply[E, A](policy: RetryPolicy)(op: IO[E, A]): IO[List[E], A] =
    def loop(remaining: RetryPolicy, reasons: List[E]): IO[List[E], A] =
      op.either.flatMap {
        case Right(a) => Eff.pure(a)
        case Left(e) if remaining.exhausted => Eff.fail((e :: reasons).reverse)
        case Left(e) => loop(remaining.next, e :: reasons)
      }
    loop(policy, Nil)
  def describe(policy: RetryPolicy): String = policy.attempts.toString + " attempts, " + policy.delay.toMillis + "ms, x" + policy.backoff
""")
    ctx.write("server.access", "Access", r"""/** The request log: one line per served call, kept bounded, summarised for the operators. */
final case class AccessLine(method: String, route: String, actor: String, site: String, millis: Long, at: Instant):
  def render: String = method + " " + route + " " + actor + "@" + site + " " + millis + "ms"

final class AccessLog(limit: Int = 500):
  private var lines: List[AccessLine] = Nil
  def add(line: AccessLine): Unit = lines = (line :: lines).take(limit)
  def all: List[AccessLine] = lines.reverse
  def count: Int = lines.length
  def byRoute: Map[String, Int] = lines.groupBy(_.route).map((k, v) => (k, v.length))
  def slowest(n: Int): List[AccessLine] = lines.sortBy(-_.millis).take(n)
  def actors: List[String] = lines.map(_.actor).distinct.sorted

object Access:
  val log: AccessLog = new AccessLog(limit = 2000)
  def record(credentials: Credentials, method: String, route: String, started: Instant): Eff[Clock & Logger, Nothing, Unit] =
    for
      clock <- Eff.service[Clock]
      millis = clock.now.toEpochMilli - started.toEpochMilli
      line = AccessLine(method, route, credentials.token.take(8), credentials.siteKey.getOrElse("global"), millis, clock.now)
      _ <- Eff.succeed(log.add(line))
      _ <- Eff.when(millis > 1000L)(Eff.service[Logger].flatMap(_.warn("access", "slow " + line.render)))
    yield ()
  def summary: String = "access=" + log.count + " routes=" + log.byRoute.size + " actors=" + log.actors.length
""")
    ctx.write("server", "Platform", r"""/** The platform pieces exercised together: settings, features, the guard, the middleware, health, metrics and retries. */
object Platform:
  def selfTest(clock: Clock, xa: Transactor, bus: Bus, logger: Logger): List[String] =
    val settings = Settings.fromEnv(Map("PROFILE" -> "staging", "HTTP_PORT" -> "9090", "FEEDS" -> "a,b", "FEATURES" -> "AuditTrail,Metrics")).fold(_ => Settings(), identity)
    val features = Features.fromSettings(settings)
    val guard = Guard(Authenticator.fixed(clock, settings.auth), features)
    val registry = new Registry(Sink.silent)
    val middleware = Middleware.standard[Task](clock, Metrics.observe(registry))
    val attempts = List(Credentials("token-1", Some("north")), Credentials("token-2", None), Credentials("token-3", None), Credentials("", None), Credentials("nope", None))
    val admitted = attempts.map { c =>
      Runtime.runEither(guard.admit(c, Method.Put, List("v1", "band", "1"))).fold(e => "denied: " + e.message, p => "admitted " + p.render)
    }
    val routed = Runtime.runEither(middleware(Request(Method.Get, List("v1", "health"), Nil, Nil, None), r => Eff.pure(ok(pathOf(r)))))
      .fold(_.toString, r => r.status.toString + " " + renderHeaders(r.headers) + " " + r.body.getOrElse(""))
    val checks = Runtime.runEither(Health.all(List(Probes.database(xa), Probes.bus(bus), Probes.logger(logger), Probes.always("disk", HealthStatus.Degraded)))).getOrElse(Nil)
    val retried = Runtime.runEither(Retry(RetryPolicy(attempts = 2))(Eff.fail("boom"))).fold(e => "gave up after " + e.mkString(","), _ => "unexpected")
    List(
      "settings: " + settings.render,
      "features: " + features.render,
      "auth: " + admitted.mkString(" | "),
      "middleware: " + routed,
      "health: " + Health.render(checks) + " -> " + Health.statusCode(Health.overall(checks)),
      "metrics: " + Metrics.summary(registry),
      "retry: " + retried + " (" + Retry.describe(RetryPolicy()) + ")",
      "access: " + Access.summary,
      "infra: " + infraLines(clock).mkString(" | ")
    )

  def infraLines(clock: Clock): List[String] =
    val mailer = new QueueMailer(clock, suppressed = Set("nobody"))
    val queue = new WorkQueue[String](clock, maxAttempts = 2)
    val cache = new MemoryCache[String, Int](clock, Duration.ofMinutes(1L), capacity = 2)
    val delivered = Runtime.runEither(
      for
        _ <- mailer.send(Mailer.compose("ops@example", Event.Created("band", "7", clock.now)))
        _ <- mailer.send(Message("nobody", "quiet", "", priority = 5))
        _ <- mailer.send(Message("local", "no domain", "body"))
        flushed <- mailer.flush
      yield flushed.map(Mailer.describe).mkString(", ")
    ).getOrElse("mail failed")
    val drained = Runtime.runEither(
      for
        _ <- queue.offerAll("a", "b", "c")
        done <- queue.drain(item => Eff.pure(if item == "b" then Ack.Retry else Ack.Done))
        left <- queue.droppedCount
      yield done.toString + " handled, " + left + " dropped"
    ).getOrElse("queue failed")
    val cached = Runtime.runEither(
      for
        a <- cache.getOrLoad("a")(Eff.pure(1))
        b <- cache.getOrLoad("b")(Eff.pure(2))
        c <- cache.getOrLoad("c")(Eff.pure(3))
        again <- cache.getOrLoad("a")(Eff.pure(10))
        n <- cache.size
      yield (a + b + c + again).toString + " sum, " + n + " kept"
    ).getOrElse("cache failed")
    List(delivered, drained, cached)
""", extra="import meridian.infra.mail.*\nimport meridian.infra.cache.*\nimport meridian.infra.queue.*\nimport meridian.server.config.*\nimport meridian.server.auth.*\nimport meridian.server.http.*\nimport meridian.server.metrics.*\nimport meridian.server.health.*\nimport meridian.server.jobs.*\nimport meridian.server.access.*")


def write_routes(ctx, area, decls, entities):
    entity = entities[0] if entities else None
    items = ctx.by_area[area]
    if entity is None or not items:
        return
    S = f"{area}Service"
    env = f"{area}Repo & Clock & Bus & Logger"
    lines = [f"/** The {area.lower()} endpoints served: credentials checked, the service called, its error mapped. */", f"object {area}Handlers:",
             f"  type R = {env}",
             "  private def guard(credentials: Credentials): IO[ServiceError, Unit] =",
             "    if credentials.token.isEmpty then Eff.fail(ServiceError.Forbidden(\"no token\")) else Eff.unit",
             "  private def scope(credentials: Credentials): String = credentials.siteKey.getOrElse(\"global\")",
             "  private def served[A](op: Eff[R, ServiceError, A]): Eff[R, ApiFailure, A] = op.mapError(_.toApi)",
             "  private def audit(credentials: Credentials, what: String): Eff[Logger, Nothing, Unit] =",
             f"    scoped(\"{area.lower()}\").info(what, \"scope\" -> scope(credentials))",
             ""]
    for e in items:
        params = [(safe(n), t) for n, t in zip([n for n, _ in e.captures] + [n for n, _ in e.queries] + ["trace" for _ in e.headers] + (["body"] if e.body is not None else []), e.inputs())]
        arity = len(params)
        if arity == 0:
            pattern = "_"
        elif arity == 1:
            pattern = params[0][0]
        else:
            pattern = "(" + ", ".join(n for n, _ in params) + ")"
        if e.action == "get":
            call = f"{S}.get(id)"
        elif e.action == "list":
            call = f"{S}.list(limit)"
        elif e.action == "create":
            call = f"{S}.create(body)"
        elif e.action == "update":
            call = f"{S}.update(id, body)"
        elif e.action == "delete":
            call = f"{S}.remove(id)"
        else:
            call = f"{S}.{e.name}({', '.join(n for n, _ in params)})"
        if arity == 0:
            signature = "(unit: Unit)"
        elif arity == 1:
            signature = f"({params[0][0]}: {e.input_type()})"
        else:
            signature = f"(input: {e.input_type()})"
        lines += [f"  private def {e.name}Logic(credentials: Credentials){signature}: Eff[R, ApiFailure, {e.output_type()}] ="]
        if arity > 1:
            lines.append(f"    val {pattern} = input")
        lines += ["    for",
                  "      _ <- served(guard(credentials))",
                  "      started <- Eff.service[Clock].map(_.now)",
                  f"      _ <- audit(credentials, \"{e.name}\")",
                  f"      result <- served({call})",
                  f"      _ <- Access.record(credentials, \"{e.method}\", \"{e.name}\", started)",
                  "    yield result",
                  f"  val {e.name}: Route[R] = Route.of({e.obj}.{e.name})(credentials => input => {e.name}Logic(credentials)(input))",
                  ""]
    names = ", ".join(e.name for e in items)
    lines += [f"  val all: List[Route[R]] = List({names})", f"  def routes: Routes[R] = Routes(all)",
              "  def templates: List[String] = all.map(_.template)",
              "  def describe(route: Route[R]): String = route.method match",
              "    case Method.Get => \"reads \" + route.template",
              "    case Method.Post => \"creates under \" + route.template",
              "    case Method.Put => \"changes \" + route.template",
              "    case Method.Delete => \"removes \" + route.template",
              "  def byMethod: Map[Method, Int] = all.groupBy(_.method).map((method, routes) => (method, routes.length))"]
    ctx.write("server.routes", f"{area}Handlers", "\n".join(lines) + "\n", extra=f"import meridian.business.*\nimport meridian.business.{area.lower()}.*\nimport meridian.server.access.*")


def write_executor(ctx):
    ctx.write("server.executor", "Jobs", '''/** Background work: jobs queued, scheduled and retried, run by the executor between requests. */
enum Job derives Eq, Show:
  case Reindex(kind: String)
  case Notify(recipient: String, text: String)
  case Cleanup(olderThanDays: Int)
  case Probe(provider: String)
  case Report(kind: String, night: LocalDate)
  def name: String = this match
    case Reindex(k) => s"reindex:$k"
    case Notify(r, _) => s"notify:$r"
    case Cleanup(d) => s"cleanup:$d"
    case Probe(p) => s"probe:$p"
    case Report(k, n) => s"report:$k:$n"
  def cost: Int = this match
    case Reindex(_) => 5
    case Notify(_, _) => 1
    case Cleanup(_) => 3
    case Probe(_) => 2
    case Report(_, _) => 4

enum Outcome derives Eq, Show:
  case Done(job: Job, millis: Long)
  case Retried(job: Job, attempt: Int)
  case Dropped(job: Job, reason: String)

final case class Scheduled(job: Job, at: Instant, attempts: Int = 0):
  def later(by: Duration): Scheduled = copy(at = at.plus(by), attempts = attempts + 1)
''')
    ctx.write("server.executor", "Executor", '''/** Runs the queued jobs in order of their time, retrying what fails up to a limit. */
final class Executor(limit: Int, clock: Clock, run: Job => Task[Unit]):
  private val queue: Ref[List[Scheduled]] = Ref.unsafeMake(Nil)
  private var outcomes: List[Outcome] = Nil

  def submit(job: Job, delay: Duration = Duration.ZERO): UIO[Unit] =
    queue.update(q => (q :+ Scheduled(job, clock.now.plus(delay))).sortBy(_.at.toEpochMilli))

  def pending: UIO[Int] = queue.get.map(_.length)

  def drain: UIO[List[Outcome]] =
    Eff.loop[Any, Nothing, (Int, Boolean)]((0, true))(_._2, state =>
      queue.modify(q => q match
        case head :: tail => (Some(head), tail)
        case Nil => (None, Nil)).flatMap {
        case Some(scheduled) => execute(scheduled).map(_ => (state._1 + 1, true))
        case None => Eff.pure((state._1, false))
      }).map(_ => outcomes.reverse)

  private def execute(scheduled: Scheduled): UIO[Unit] =
    val start = clock.now
    run(scheduled.job).foldEff(
      failure =>
        if scheduled.attempts < limit then
          record(Outcome.Retried(scheduled.job, scheduled.attempts + 1)) *> queue.update(q => (q :+ scheduled.later(Duration.ofMinutes(5L))).sortBy(_.at.toEpochMilli))
        else record(Outcome.Dropped(scheduled.job, failure.getMessage)),
      _ => record(Outcome.Done(scheduled.job, Duration.between(start, clock.now).toMillis)))

  private def record(outcome: Outcome): UIO[Unit] = Eff.succeed { outcomes = outcome :: outcomes }

  def summary: String =
    val done = outcomes.count { case Outcome.Done(_, _) => true; case _ => false }
    val retried = outcomes.count { case Outcome.Retried(_, _) => true; case _ => false }
    val dropped = outcomes.count { case Outcome.Dropped(_, _) => true; case _ => false }
    s"done=$done retried=$retried dropped=$dropped"

''')
    ctx.write("server.executor", "Scheduler", '''/** Recurring jobs on a fixed grid. */
final case class Rule(job: Job, every: Duration, last: Option[Instant] = None):
  def due(now: Instant): Boolean = last.forall(l => !l.plus(every).isAfter(now))
  def ran(now: Instant): Rule = copy(last = Some(now))

final class Scheduler(initial: List[Rule]):
  private val state: Ref[List[Rule]] = Ref.unsafeMake(initial)
  def tick(now: Instant, executor: Executor): UIO[Int] =
    state.modify { current =>
      val (due, rest) = current.partition(_.due(now))
      (due, due.map(_.ran(now)) ++ rest)
    }.flatMap(due => Eff.foreachDiscard(due)(r => executor.submit(r.job)).as(due.length))
  def rules: UIO[List[Rule]] = state.get

object Scheduler:
  def standard(providers: List[String]): Scheduler =
    new Scheduler(
      Rule(Job.Cleanup(30), Duration.ofHours(24L)) ::
        Rule(Job.Reindex("all"), Duration.ofHours(6L)) ::
        providers.map(p => Rule(Job.Probe(p), Duration.ofMinutes(15L))))
''')


def write_wiring(ctx):
    lines = ["/** Everything the routes need, built from the samples and the in-memory infrastructure. */", "object Wiring:",
             "  final case class Services("]
    for area, decls, entities in ctx.areas:
        if entities:
            lines.append(f"    {lower(area)}: {area}Repo,")
    lines += ["    xa: Transactor,", "    bus: Bus,", "    logger: Logger,", "    clock: Clock,", "    blobs: Blobs,", "    index: Index", "  )", "",
              "  def build(clock: Clock): Services =", "    val xa = Transactor.make", "    Services("]
    for area, decls, entities in ctx.areas:
        if entities:
            entity = entities[0]
            lines.append(f"      {area}Repo.seeded(List({area}Samples.{entity.sample_name()}), xa),")
    lines += ["      xa, Bus.make, Logger.make(), clock, Blobs.make(), Index.make)", ""]
    lines.append("  type ServerEnv = " + " & ".join([f"{area}Repo" for area, _, entities in ctx.areas if entities] + ["Transactor", "Bus", "Logger", "Clock", "Blobs", "Index"]))
    lines += ["", "  def env(services: Services): Env[ServerEnv] =", "    Env.empty"]
    for area, decls, entities in ctx.areas:
        if entities:
            lines.append(f"      .add(services.{lower(area)})")
    lines += ["      .add(services.xa)", "      .add(services.bus)", "      .add(services.logger)", "      .add(services.clock)", "      .add(services.blobs)", "      .add(services.index)", ""]
    lines += ["  def routes: Routes[ServerEnv] =", "    Routes(List("]
    lines.append(",\n".join(f"      {area}Handlers.all" for area, _, entities in ctx.areas if entities and ctx.by_area[area]) + ").flatten)")
    ctx.write("server", "Wiring", "\n".join(lines) + "\n", extra="import meridian.business.*\nimport meridian.server.routes.*\nimport meridian.model.samples.*\nimport meridian.infra.storage.*\nimport meridian.infra.search.*\n" + "\n".join(f"import meridian.business.{area.lower()}.*" for area, _, e in ctx.areas if e))


def write_views(ctx):
    ctx.write("server.view", "Report", '''/** Text renderings of what the self-test and an operator read. */
object Report:
  def table(headers: List[String], rows: List[List[String]]): String = Text.column(headers :: rows)
  def kv(pairs: List[(String, String)]): String = pairs.map((k, v) => s"$k: $v").mkString("\\n")
  def events(bus: Bus, limit: Int): String = bus.history.takeRight(limit).map(_.describe).mkString("\\n")
  def statements(xa: Transactor, limit: Int): String = xa.seen.take(limit).mkString("\\n")
  def logs(logger: Logger, limit: Int): String = logger.drain.take(limit).map(_.render).mkString("\\n")
  def counts(m: Map[String, Int]): String = m.toList.sortBy(_._1).map((k, v) => s"$k=$v").mkString(", ")
''')
    ctx.write("server.view", "Health", '''/** The health page: providers probed, jobs summarised. */
final case class HealthPage(providers: List[ProviderReport], jobs: String, events: Int, statements: Int):
  def render: String =
    (providers.map(_.render) :+ s"jobs: $jobs" :+ s"events: $events" :+ s"statements: $statements").mkString("\\n")
  def healthy: Boolean = providers.forall(_.healthy)
''', extra="import meridian.infra.feeds.*")


def write_main(ctx):
    rng = ctx.rng
    entity_areas = [(area, decls, entities) for area, decls, entities in ctx.areas if entities and ctx.by_area[area]]
    lines = ['''/** The self-test of the API: every endpoint receives a request built from the samples, the
  * response is printed for the first few and hashed for all, then the services, the reports,
  * the providers and the executor run, and a checksum ends the output. */
object Main:
  private val out = Checksum.Builder()
  private val credentials = Credentials("token-1", Some("north"))
  private var handle: Request => Response = _ => Response(500, Nil, None)
  private var shown = 0

  def send(request: Request): Unit =
    val response = handle(request)
    val line = s"${request.render} -> ${response.render}"
    out.add(line)
    if shown < 40 then
      println(line.take(220))
      shown += 1

  def main(args: Array[String]): Unit =
    val clock = Clock.ticking(Instant.ofEpochSecond(1700000000L), Duration.ofSeconds(7L))
    val services = Wiring.build(clock)
    val env = Wiring.env(services)
    val routes = Wiring.routes
    handle = request => Runtime.run(routes.handle(request), env).getOrThrow
    println(s"${routes.routes.length} routes")
''']
    body = []
    for area, decls, entities in entity_areas:
        for e in ctx.by_area[area]:
            body.append(f"    send(Client.render({e.ref()})(credentials)({e.input_sample(rng)}))")
    lines.append("    requests0()")
    groups = [body[i : i + 50] for i in range(0, len(body), 50)]
    lines += [f"    requests{i}()" for i in range(1, len(groups))]
    lines += ['''    send(Request(Method.Get, List("v1", "nowhere"), Nil, List(("X-Token", "t")), None))
    send(Request(Method.Get, List("v1"), Nil, Nil, None))
    services_(env, services)
    providers(clock)
    executor(clock, services)
    allocation()
    Platform.selfTest(services.clock, services.xa, services.bus, services.logger).foreach(println)
    println(s"statements: ${services.xa.summary}")
    println(s"events: ${Report.counts(services.bus.kinds)}")
    println(s"log lines: ${services.logger.count}")
    println(s"checksum ${out.value} over ${out.lines} lines")

  def services_(env: Env[Wiring.ServerEnv], services: Wiring.Services): Unit =''']
    svc = []
    for area, decls, entities in entity_areas[:20]:
        entity = entities[0]
        svc.append(f"    stat(\"{area.lower()}\", Runtime.run({area}Service.stats, env).toEither.map(_.render))")
        svc.append(f"    stat(\"{area.lower()} report\", Runtime.run({area}Service.list(Some(5)).map({area}Reports.render), env).toEither)")
        svc.append(f"    stat(\"{area.lower()} create\", Runtime.run({area}Service.create({area}Samples.{entity.sample_name()}), env).toEither.map(_ => \"conflict expected\").left.map(_.toString))")
        svc.append(f"    stat(\"{area.lower()} export\", Runtime.run({area}Service.list(Some(2)).map(items => {area}Export.csv(items).length.toString + \" csv chars, \" + {area}Export.fixed(items).length + \" fixed chars\"), env).toEither.left.map(_.toString))")
        svc.append(f"    stat(\"{area.lower()} digest\", Right({area}Notifications.digest(services.bus.history).length.toString + \" chars, urgency \" + services.bus.history.map({area}Notifications.urgency).sum))")
        svc.append(f"    stat(\"{area.lower()} listing\", Runtime.run({area}Service.listing({area}Queries.fromParams(Map(\"size\" -> \"3\", \"dir\" -> \"desc\"))).map(_.map(_.id.raw.toString).mkString(\",\")), env).toEither.left.map(_.toString))")
        svc.append(f"    stat(\"{area.lower()} change\", Runtime.run({area}Service.change({area}Samples.{entity.sample_name()}.id, {area}Samples.{entity.sample_name()}, \"tester\", note = Some(\"checked\")).flatMap(changes => {area}Service.perform(\"history\", {area}Samples.{entity.sample_name()}.id, \"tester\").map(h => changes.length.toString + \" changes, \" + h)), env).toEither.left.map(_.toString))")
    lines += svc
    lines += ['''    println(s"services checked: ${checked}")

  private var checked = 0
  def stat(name: String, result: Either[Any, String]): Unit =
    checked += 1
    val text = result.fold(e => s"$name: error $e", ok => s"$name: $ok")
    out.add(text)
    if checked <= 12 then println(text.take(200))

  def providers(clock: Clock): Unit =
    val reports = List(''']
    lines.append(",\n".join(f"      Providers.report({p}Client.canned(clock), Instant.ofEpochSecond(1600000000L))" for p in ctx.providers))
    lines += ['''    )
    val page = HealthPage(reports.map(r => Runtime.unsafeRun(r).getOrThrow), "none yet", 0, 0)
    page.render.split("\\n").foreach(l => out.add(l))
    println(page.render.split("\\n").take(4).mkString("\\n"))
    println(s"providers healthy: ${page.healthy}")
    println("fixtures decoded: " + List(''' + ', '.join(f'{p}Fixtures.decodedStations.fold(_ => -1, _.length)' for p in ctx.providers) + ''').mkString(","))

  def executor(clock: Clock, services: Wiring.Services): Unit =
    val executor = new Executor(2, clock, job => job match
      case Job.Probe(p) if p.startsWith("z") => Eff.fail(new IllegalStateException(s"no such provider $p"))
      case Job.Reindex(kind) => services.index.add(Document(kind, "kind", Map("text" -> kind)))
      case other => services.logger.info("executor", other.name))
    val scheduler = Scheduler.standard(List(''' + ", ".join(f'"{p}"' for p in ctx.providers[:3]) + ''', "zeta"))
    val run =
      for
        _ <- executor.submit(Job.Notify("ada", "night starts"))
        _ <- executor.submit(Job.Cleanup(7), Duration.ofMinutes(1L))
        due <- scheduler.tick(clock.now, executor)
        pending <- executor.pending
        outcomes <- executor.drain
        _ <- scheduler.tick(clock.now.plus(Duration.ofHours(7L)), executor)
        more <- executor.drain
      yield (due, pending, outcomes.length + more.length)
    val (due, pending, total) = Runtime.unsafeRun(run).getOrThrow
    val summary = s"scheduled $due, pending $pending, outcomes $total, ${executor.summary}"
    out.add(summary)
    println(summary)

  def allocation(): Unit =
    val quotas = List(Quota("alpha", 40, 3), Quota("beta", 25, 2), Quota("gamma", 10, 1))
    val summary = Summary.of(LocalDate.of(2026, 3, 1), 30, quotas, Tariff.standard)
    val budgets = quotas.map(q => (q.proposal, Budget.of(Amount.ofUnits(q.hours.toLong * 50L)))).toMap
    val plan = Planner.fill(quotas.map(q => (q.proposal, InstrumentClass.Spectrograph, q.minutes / 10L)), Period.Dark, LocalDate.of(2026, 3, 2))
    val check = Planner.check(plan, budgets, Tariff.standard)
    val charged = Planner.charge(plan, budgets, Tariff.standard)
    val lines = List(summary.render, s"plan ${plan.minutes} minutes, ${check.faults.length} faults", s"charged: ${charged.map(_.toList.sortBy(_._1).map((p, b) => s"$p=${b.usedShare}%").mkString(",")).left.map(_.toString)}", Quota.summary(quotas), Calendar.label(LocalDate.of(2026, 3, 9)))
    lines.foreach(l => out.add(l))
    lines.foreach(println)
''']
    for i, group in enumerate(groups):
        lines.append("")
        lines.append(f"  def requests{i}(): Unit =")
        lines += group
        lines.append('''    ()''')
    if not groups:
        lines += ["", "  def requests0(): Unit = ()"]
    extra = "import meridian.business.*\nimport meridian.server.routes.*\nimport meridian.server.executor.*\nimport meridian.server.view.*\nimport meridian.model.samples.*\nimport meridian.infra.feeds.*\nimport meridian.infra.search.*\nimport meridian.allocation.*\n" + "\n".join(f"import meridian.infra.feeds.{p.lower()}.*" for p in ctx.providers) + "\n" + "\n".join(f"import meridian.business.{area.lower()}.*" for area, _, e in entity_areas)
    ctx.write("server", "Main", "\n".join(lines) + "\n", extra=extra)
