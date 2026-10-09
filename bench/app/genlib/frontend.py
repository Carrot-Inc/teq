"""The frontend: the browser application over the model. Per area a state slice with its
lenses, an api client, actions over the effect type, components (cards, tables, editors,
filters, badges) with class-name macro sites, pages that compose them, a router and a self-test
that renders the pages, fires events and prints a checksum."""
import os
import re

from . import frontend_area

HEADER = """package {pkg}

import meridian.core.*
import meridian.core.Codecs.given
import meridian.core.http.{{Request, Response}}
import meridian.web.*
import meridian.model.*
import meridian.model.api.*
import meridian.frontend.main.*
import meridian.frontend.apiclient.*
import cats.syntax.all.*
{extra}
"""


class Ctx:
    def __init__(self, u, endpoints, areas, out, rng, flags, scale, macros):
        self.u = u
        self.endpoints = endpoints
        self.areas = areas
        self.out = os.path.join(out, "frontend", "frontend")
        self.rng = rng
        self.flags = flags
        self.scale = scale
        self.macros = macros
        self.files = 0
        self.cls_sites = 0
        self.tw_sites = 0
        self.names_sites = 0
        self.by_area = {area: [e for e in endpoints if e.area == area] for area, _, _ in areas}
        self.pages = []
        self.flag = None

    def write(self, pkg, name, body, extra=""):
        d = os.path.join(self.out, *pkg.split(".")) if pkg else self.out
        os.makedirs(d, exist_ok=True)
        with open(os.path.join(d, name + ".scala"), "w") as f:
            f.write(HEADER.format(pkg="meridian.frontend" + ("." + pkg if pkg else ""), extra=extra) + "\n" + spaced(spread_params(body)))
        self.files += 1

    def classes(self, n=None):
        """A `cls :=` site over a valid class string, its literals split into a few arguments; now and
        then a plain class attribute instead, as pages have."""
        used = set()
        tokens = self.macros["class_string"](used).split(" ")
        if self.rng.chance(0.18):
            return f'vdomCls := "{" ".join(tokens)}"'
        self.cls_sites += 1
        if n is None:
            n = self.rng.weighted([(1, 4), (2, 3), (3, 2)])
        parts = []
        for i, chunk in enumerate(chunks(tokens, n)):
            text = " ".join(chunk)
            kind = self.rng.weighted([("lit", 6), ("pair", 2 if self.flag else 0), ("tw", 2)]) if i > 0 else "lit"
            if kind == "pair":
                parts.append(f'"{text}" -> {self.flag}')
            elif kind == "tw":
                self.tw_sites += 1
                parts.append(f'tw"{text}"')
            else:
                parts.append(f'"{text}"')
        if len(parts) == 1:
            return f"cls := {parts[0]}"
        return f"cls := ({', '.join(parts)})"

    def tw(self):
        """A `tw` literal, or the `classNames` call or the unchecked constructor a page falls back to."""
        used = set()
        text = self.macros["class_string"](used)
        kind = self.rng.weighted([("tw", 82), ("unsafe", 13), ("names", 5)])
        if kind == "unsafe":
            return f'Tw.unsafe("{text}")'
        if kind == "names":
            self.names_sites += 1
            return "classNames(" + ", ".join(f'"{t}"' for t in text.split(" ")) + ")"
        self.tw_sites += 1
        return f'tw"{text}"'

    def class_names(self):
        used = set()
        self.names_sites += 1
        tokens = self.macros["class_string"](used).split(" ")
        return "classNames(" + ", ".join(f'"{t}"' for t in tokens) + ")"

    def show_text(self, expr):
        return f'show"{expr}"' if self.flags.show_interpolator else f's"{expr}"'


def split_params(text):
    """The parameters of a declaration, split at the commas outside brackets and strings."""
    parts = []
    depth = 0
    quoted = False
    current = ""
    for c in text:
        if c == '"':
            quoted = not quoted
        elif not quoted and c in "[({":
            depth += 1
        elif not quoted and c in "])}":
            depth -= 1
        if c == "," and depth == 0 and not quoted:
            parts.append(current.strip())
            current = ""
        else:
            current += c
    if current.strip():
        parts.append(current.strip())
    return parts


CASE_CLASS_LINE = re.compile(r"^(\s*)(final case class \w+(?:\[[^\]]*\])?)\((.*)\)(.*)$")


def spread_params(body):
    """Case classes of three or more fields declared one field per line, as the pages write them."""
    out = []
    for line in body.split("\n"):
        m = CASE_CLASS_LINE.match(line)
        if m and "(" not in m.group(2):
            indent, head, params, tail = m.groups()
            fields = split_params(params)
            if len(fields) >= 3 and all(": " in f for f in fields):
                out.append(f"{indent}{head}(")
                out += [f"{indent}  {f}," for f in fields[:-1]] + [f"{indent}  {fields[-1]}"]
                out.append(f"{indent}){tail}")
                continue
        out.append(line)
    return "\n".join(out)


def spaced(body):
    """A blank line before every top-level definition and before the node a component returns."""
    out = []
    for line in body.split("\n"):
        top = line[:1] not in (" ", "\t", "") and any(line.startswith(k) for k in ("val ", "def ", "final case class ", "enum ", "object ", "/**"))
        returns = line.startswith("  ") and not line.startswith("   ") and line.lstrip()[:1].islower() and "(" in line and out and out[-1].lstrip().startswith(("val ", "def ", "case ")) and not line.lstrip().startswith(("val ", "def ", "case "))
        if (top or returns) and out and out[-1].strip():
            out.append("")
        out.append(line)
    return "\n".join(out)


def chunks(items, n):
    n = max(1, min(n, len(items)))
    size = (len(items) + n - 1) // n
    return [items[i : i + size] for i in range(0, len(items), size)]


def lower(name):
    return name[0].lower() + name[1:]


def write_frontend(u, endpoints, areas, out, targets, rng, flags, scale, macros):
    ctx = Ctx(u, endpoints, areas, out, rng, flags, scale, macros)
    write_environment(ctx)
    write_states(ctx)
    write_api_clients(ctx)
    write_actions(ctx)
    write_widgets(ctx)
    for area, decls, entities in areas:
        frontend_area.write_area(ctx, area, decls, entities)
    write_router(ctx)
    write_backend(ctx)
    write_main(ctx)
    return {"files": ctx.files, "cls": ctx.cls_sites, "tw": ctx.tw_sites, "classNames": ctx.names_sites}


# ---------------------------------------------------------------------------------------------
# main: environment, state, lenses, actions

def write_environment(ctx):
    ctx.write("main", "Environment", '''/** What every action runs in: the transport to the backend, the session and the clock. */
trait Clockwork:
  def now: Instant
  def today: LocalDate
object Clockwork:
  def fixed(at: Instant): Clockwork = new Clockwork:
    def now = at
    def today = at.toLocalDate

type ConsoleEnv = Transport & Session & Clockwork
type AppTask[A] = Eff[ConsoleEnv, Throwable, A]
type ApiTask[A] = Eff[Transport & Session, Nothing, ApiResult[A]]

object Environment:
  val store: Store[FrontendState] = Store(FrontendState.initial)
  def toast(text: String): UIO[Unit] = store.modL(ConsoleLenses.toasts)(ts => (ts :+ text).takeRight(8))
  def session: Eff[Session, Nothing, Session] = Eff.service[Session]
  def now: Eff[Clockwork, Nothing, Instant] = Eff.service[Clockwork].map(_.now)
  def busy(on: Boolean): UIO[Unit] = store.setL(ConsoleLenses.busy)(on)
  def track(event: String): UIO[Unit] = store.modL(ConsoleLenses.events)(es => event :: es)

/** A page holds a runner; a component gets it through its props. */
final case class Actions(runner: Runner[ConsoleEnv]):
  def run(action: Eff[ConsoleEnv, Throwable, Any]): Callback = runner.runAction(action)
''')
    ctx.write("main", "FrontendState", "\n".join(state_root(ctx)) + "\n", extra="import meridian.frontend.main.state.*")


def area_state_name(area):
    return f"{area}State"


def state_root(ctx):
    lines = ["/** The whole state of the frontend: one slice per area, and what the shell shows. */", "final case class FrontendState("]
    for area, _, _ in ctx.areas:
        lines.append(f"  {lower(area)}: {area_state_name(area)},")
    lines += ["  toasts: List[String],", "  busy: Boolean,", "  events: List[String],", "  currentUser: Option[String]", ")", "", "object FrontendState:", "  val initial: FrontendState = FrontendState("]
    for area, _, _ in ctx.areas:
        lines.append(f"    {area_state_name(area)}.initial,")
    lines += ["    Nil, false, Nil, None)", "", "object ConsoleLenses:"]
    for area, _, _ in ctx.areas:
        lines.append(f"  val {lower(area)} = Lens.gen[FrontendState](_.{lower(area)})")
    lines += ["  val toasts = Lens.gen[FrontendState](_.toasts)", "  val busy = Lens.gen[FrontendState](_.busy)", "  val events = Lens.gen[FrontendState](_.events)", "  val currentUser = Lens.gen[FrontendState](_.currentUser)"]
    return lines


def write_states(ctx):
    for area, decls, entities in ctx.areas:
        entity = entities[0] if entities else None
        enums = [d for d in decls if d.kind == "enum"]
        st = area_state_name(area)
        lines = [f"/** The slice of the state that the {area.lower()} pages read and write. */", f"final case class {st}("]
        fields = []
        if entity is not None:
            idn = entity.entity_id.name
            fields += [f"items: Map[{idn}, {entity.name}]", f"selected: Option[{idn}]", f"draft: Option[{entity.name}]", "loading: Boolean", "errors: List[Fault]", f"order: List[{idn}]"]
        else:
            fields += ["loading: Boolean", "errors: List[Fault]", "notes: List[String]"]
        if enums:
            fields.append(f"filter: Option[{enums[0].name}]")
        fields.append("search: String")
        fields.append("page: Int")
        if ctx.rng.chance(0.5):
            fields.append("expanded: Set[String]")
        lines += [f"  {f}," for f in fields[:-1]] + [f"  {fields[-1]}"]
        lines.append(") derives Eq:" if entity is None or entity.has("Eq") else "):")
        if entity is not None:
            lines.append(f"  def selectedItem: Option[{entity.name}] = selected.flatMap(items.get)")
            lines.append(f"  def visible: List[{entity.name}] = order.flatMap(items.get)")
            lines.append("  def count: Int = items.size")
            lines.append("  def isEmpty: Boolean = items.isEmpty")
            lines.append(f"  def has(id: {entity.entity_id.name}): Boolean = items.contains(id)")
            lines.append(f"  def withItem(item: {entity.name}): {st} = copy(items = items.updated(item.id, item), order = if order.contains(item.id) then order else order :+ item.id)")
            lines.append(f"  def without(id: {entity.entity_id.name}): {st} = copy(items = items.removed(id), order = order.filterNot(_ == id), selected = selected.filterNot(_ == id))")
            lines.append("  def phase: String = (loading, errors.nonEmpty, items.isEmpty) match")
            lines.append("    case (true, _, _) => \"loading\"")
            lines.append("    case (_, true, _) => \"failed\"")
            lines.append("    case (_, _, true) => \"empty\"")
            lines.append("    case _ => \"ready\"")
        lines.append("  def hasErrors: Boolean = errors.nonEmpty")
        lines.append("  def pageOf(size: Int): Int = if size <= 0 then 0 else page * size")
        lines.append("  def searching: Boolean = search.nonEmpty")
        lines.append("")
        lines.append(f"object {st}:")
        init = []
        for f in fields:
            name, tpe = f.split(": ", 1)
            init.append({"Map": "Map.empty", "Option": "None", "Boolean": "false", "List": "Nil", "String": '""', "Int": "0", "Set": "Set.empty"}[tpe.split("[")[0]])
        lines.append(f"  val initial: {st} = {st}({', '.join(init)})")
        for f in fields:
            name = f.split(":")[0]
            lines.append(f"  val {name} = Lens.gen[{st}](_.{name})")
        if entity is not None:
            lines.append(f"  def itemAt(id: {entity.entity_id.name}): Optional[{st}, {entity.name}] = items.index(id)")
            lines.append(f"  val draftSome: Optional[{st}, {entity.name}] = draft.some")
        lines.append("")
        lines.append(f"object {area}Lenses:")
        for f in fields:
            name = f.split(":")[0]
            lines.append(f"  val {name} = ConsoleLenses.{lower(area)}.andThen({st}.{name})")
        ctx.write("main.state", st, "\n".join(lines) + "\n")


def endpoint_params(e):
    names = [n for n, _ in e.captures] + [n for n, _ in e.queries] + ["trace" for _ in e.headers] + (["body"] if e.body is not None else [])
    return [(safe(n), t) for n, t in zip(names, e.inputs())]


def safe(name):
    return name if name.isidentifier() and name not in ("type",) else name.replace("-", "").lower() + "Value"


def write_api_clients(ctx):
    for area, decls, entities in ctx.areas:
        items = ctx.by_area[area]
        if not items:
            continue
        lines = [f"/** The {area.lower()} endpoints as effects, each with a logged and a retried form. */", f"object {area}Api:",
                 "  private def logged[A](name: String)(call: ApiTask[A]): ApiTask[A] =",
                 "    call.tap(result => Log.infoEff(s\"" + area.lower() + " $name: ${result.fold(_.describe, _ => \"ok\")}\"))",
                 "  private def retried[A](call: ApiTask[A]): ApiTask[A] =",
                 "    call.flatMap(result => if result.isLeft then call else Eff.pure(result))", ""]
        for e in items:
            out_t = e.output_type()
            params = endpoint_params(e)
            if not params:
                lines.append(f"  def {e.name}: ApiTask[{out_t}] = ApiClient.call({e.ref()})(())")
                lines.append(f"  def {e.name}Logged: ApiTask[{out_t}] = logged(\"{e.name}\")({e.name})")
            else:
                sig = ", ".join(f"{n}: {t.expr}" for n, t in params)
                args = [n for n, _ in params]
                arg = args[0] if len(args) == 1 else "(" + ", ".join(args) + ")"
                lines.append(f"  def {e.name}({sig}): ApiTask[{out_t}] = ApiClient.call({e.ref()})({arg})")
                lines.append(f"  def {e.name}Logged({sig}): ApiTask[{out_t}] = logged(\"{e.name}\")({e.name}({', '.join(args)}))")
                if e.action in ("get", "list"):
                    lines.append(f"  def {e.name}Retried({sig}): ApiTask[{out_t}] = retried({e.name}({', '.join(args)}))")
            lines.append("")
        ctx.write("apiclient", f"{area}Api", "\n".join(lines) + "\n")


def write_actions(ctx):
    for area, decls, entities in ctx.areas:
        items = ctx.by_area[area]
        entity = entities[0] if entities else None
        L = f"{area}Lenses"
        enums = [d for d in decls if d.kind == "enum"]
        lines = [f"/** What the {area.lower()} pages do. */", f"object {area}Actions:", "  import Environment.*", ""]
        if entity is not None:
            en = entity.name
            idn = entity.entity_id.name
            get = next((e for e in items if e.action == "get"), None)
            lst = next((e for e in items if e.action == "list"), None)
            create = next((e for e in items if e.action == "create"), None)
            update = next((e for e in items if e.action == "update"), None)
            delete = next((e for e in items if e.action == "delete"), None)
            if get is not None:
                lines += [f"  def load(id: {idn}): AppTask[Unit] =", "    for", "      _ <- busy(true)", f"      result <- {area}Api.{get.name}(id)", "      _ <- result match",
                          f"        case Right(item) => store.modL({L}.items)(_.updated(id, item)) *> store.setL({L}.selected)(Some(id))",
                          "        case Left(error) => toast(error.describe)", "      _ <- busy(false)", f'      _ <- track(s"load {area.lower()} ${{id.raw}}")', "    yield ()", ""]
            if lst is not None:
                args = ", ".join(["None"] * len(lst.inputs()))
                lines += ["  def refresh: AppTask[Int] =", "    for", f"      result <- {area}Api.{lst.name}({args})", "      items = result.getOrElse(Nil)",
                          f"      _ <- store.modL({L}.items)(_ ++ items.map(x => (x.id, x)))", f"      _ <- store.setL({L}.order)(items.map(_.id))",
                          "      _ <- Eff.when(items.isEmpty)(toast(\"nothing to show\"))", "    yield items.length", ""]
            lines += [f"  def select(id: Option[{idn}]): UIO[Unit] = store.setL({L}.selected)(id)",
                      f"  def edit(item: {en}): UIO[Unit] = store.setL({L}.draft)(Some(item)) *> track(\"edit\")",
                      f"  def discard: UIO[Unit] = store.setL({L}.draft)(None) *> store.setL({L}.errors)(Nil)", ""]
            if create is not None or update is not None:
                save_call = f"{area}Api.{update.name}(item.id, item).map(_.map(_ => item))" if update is not None else f"{area}Api.{create.name}(item)"
                lines += ["  def save: AppTask[Unit] =", "    for", f"      draft <- store.read({L}.draft)", "      _ <- draft match", "        case None => toast(\"nothing to save\")",
                          "        case Some(item) =>", "          for", f"            saved <- {save_call}", "            _ <- saved match",
                          f"              case Right(value) => store.modL({L}.items)(_.updated(item.id, value)) *> discard *> toast(\"saved\")",
                          f"              case Left(failure) => store.setL({L}.errors)(List(Fault(\"save\", failure.describe)))", "          yield ()", "    yield ()", ""]
            if delete is not None:
                lines += [f"  def remove(id: {idn}): AppTask[Unit] =",
                          f"    {area}Api.{delete.name}(id).flatMap(result =>",
                          f"      Eff.when(result.isRight)(store.modL({L}.items)(_.removed(id)) *> store.modL({L}.order)(_.filterNot(_ === id))) *>",
                          "        Eff.when(result.isLeft)(toast(\"could not remove\")))", ""]
            lines += [f"  def selectNext: UIO[Unit] =", "    for", f"      slice <- store.read(ConsoleLenses.{lower(area)})",
                      f"      next = {area}Helpers.selectNext(slice.order, slice.selected)", f"      _ <- store.setL({L}.selected)(next)", "    yield ()", "",
                      f"  def toggleExpanded(key: String): UIO[Unit] = store.modL({L}.expanded)(set => if set.contains(key) then set - key else set + key)" if any("expanded" in f for f in []) else f"  def clearErrors: UIO[Unit] = store.setL({L}.errors)(Nil)",
                      f"  def loadAll(ids: List[{idn}]): AppTask[Int] =", "    for", "      _ <- busy(true)", "      loaded <- Eff.foreach(ids)(id => load(id).either)", "      _ <- busy(false)",
                      "      count = loaded.count(_.isRight)", f'      _ <- toast(s"loaded $count of ${{ids.length}}")', "    yield count", "",
                      f"  def removeMany(ids: List[{idn}]): AppTask[Int] =", "    for", "      results <- Eff.foreach(ids)(id => remove(id).either)", "      failures = results.count(_.isLeft)",
                      "      _ <- Eff.when(failures > 0)(toast(s\"$failures failed\"))", "    yield ids.length - failures", "",
                      f"  def refreshIfStale(limit: Int): AppTask[Boolean] =",
                      f"    store.read(ConsoleLenses.{lower(area)}).flatMap(slice => Eff.when(slice.count < limit)(refresh).as(slice.count < limit))",
                      f"  def rename(id: {idn}, text: String): AppTask[Unit] =",
                      f"    store.read(ConsoleLenses.{lower(area)}).flatMap(slice => slice.items.get(id) match",
                      f"      case Some(item) => edit(item) *> track(s\"rename ${{id.raw}} to $text\")",
                      "      case None => toast(\"not loaded\"))", ""] if get is not None and delete is not None and lst is not None else []
            for e in [e for e in items if e.action == "custom"][:1]:
                params = endpoint_params(e)
                sig = ", ".join(f"{n}: {t.expr}" for n, t in params)
                args = [n for n, _ in params]
                reload = f"      _ <- Eff.when(ok)(load({args[0]}))" if e.output is not None and e.captures else "      _ <- Eff.unit"
                lines += [f"  def {e.name}({sig}): AppTask[Boolean] =", "    for", f"      result <- {area}Api.{e.name}({', '.join(args)})",
                          f"      _ <- track(\"{e.name}\")", "      ok = result.isRight", f"      _ <- Eff.unless(ok)(toast(\"failed: {e.name}\"))", reload, "    yield ok", ""]
            if enums:
                lines.append(f"  def setFilter(value: Option[{enums[0].name}]): UIO[Unit] = store.setL({L}.filter)(value)")
            else:
                lines.append(f"  def clearSearch: UIO[Unit] = store.setL({L}.search)(\"\")")
            lines += [f"  def search(text: String): UIO[Unit] = store.setL({L}.search)(text) *> store.setL({L}.page)(0)",
                      f"  def nextPage: UIO[Unit] = store.modL({L}.page)(_ + 1)",
                      f"  def validate(item: {en}): Check[{en}] =",
                      f"    Check.map2(Rules.positive(\"id\", item.id.raw), Check.valid(item))((_, x) => x)" if entity.entity_id.raw_type == "Long" else f"    Check.map2(Rules.nonEmpty(\"id\", item.id.raw), Check.valid(item))((_, x) => x)"]
        else:
            lines += [f"  def note(text: String): UIO[Unit] = store.modL({L}.notes)(text :: _)", f"  def clear: UIO[Unit] = store.setL({L}.notes)(Nil)",
                      f"  def search(text: String): UIO[Unit] = store.setL({L}.search)(text)"]
        ctx.write("main.actions", f"{area}Actions", "\n".join(lines) + "\n", extra=f"import meridian.frontend.main.state.*\nimport meridian.frontend.page.{area.lower()}.*")


# ---------------------------------------------------------------------------------------------
# widgets: generic components shared by the pages

WIDGETS = ["Button", "Badge", "Tabs", "Modal", "Dropdown", "SearchBox", "Pagination", "Table", "EmptyState", "KeyValue", "Toasts", "PageShell", "Breadcrumbs", "Spinner", "DateLabel", "AmountLabel", "TextField", "NumberField", "Toggle", "Stat",
           "IconButton", "Chip", "Alert", "Avatar", "Tooltip", "Card", "Drawer", "Stepper", "Progress", "Checkbox", "Sidebar", "Topbar", "FooterBar", "SplitPane", "Timeline", "Sparkline", "SectionHeader", "Toolbar", "Legend", "Ribbon",
           "Callout", "Divider", "Skeleton", "Kbd", "Tag", "Meter", "Rating", "Counter", "Switch", "Slider", "Accordion", "Popover", "Menu", "Notice", "Pill"]


WIDGET_NAMES = {"Button": "actionButton", "Table": "dataTable", "Progress": "progressBar", "Legend": "legendBox"}


def widget_name(w):
    return WIDGET_NAMES.get(w, lower(w))


def write_widgets(ctx):
    for w in WIDGETS:
        ctx.flag = "hovered" if w not in ("Button", "Badge", "Tabs", "Modal", "Dropdown", "SearchBox", "Pagination", "Table", "EmptyState", "KeyValue", "Toasts", "PageShell", "Breadcrumbs", "Spinner", "DateLabel", "AmountLabel", "TextField", "NumberField", "Toggle", "Stat", "Chip") else None
        ctx.write("component.widget", w, widget_body(ctx, w, widget_name(w)))
        ctx.flag = None


def widget_body(ctx, w, name):
    rng = ctx.rng
    c = ctx.classes
    if w == "Button":
        return f'''enum ButtonTone derives Enumerated, Eq:
  case Primary, Secondary, Danger, Ghost
enum ButtonScale(val label: String) derives Enumerated, Eq:
  case Small extends ButtonScale("sm")
  case Medium extends ButtonScale("md")
  case Large extends ButtonScale("lg")

final case class ButtonProps(text: String, onClick: Callback, intent: ButtonTone = ButtonTone.Primary, size: ButtonScale = ButtonScale.Medium, disabled: Boolean = false, busy: Boolean = false)
object ButtonProps:
  given Reusability[ButtonProps] = Reusability.by(p => (p.text, p.disabled, p.busy))

val actionButton: FC[ButtonProps] = FC[ButtonProps]: props =>
  val intent = props.intent match
    case ButtonTone.Primary => {ctx.tw()}
    case ButtonTone.Secondary => {ctx.tw()}
    case ButtonTone.Danger => {ctx.tw()}
    case ButtonTone.Ghost => {ctx.tw()}
  val size = props.size match
    case ButtonScale.Small => {ctx.tw()}
    case ButtonScale.Medium => {ctx.tw()}
    case ButtonScale.Large => {ctx.tw()}
  Tags.button({c()}, cls :=? Some(concatClasses(List(intent, size))), disabled := (props.disabled || props.busy), onClick --> props.onClick,
    span({c()}).when(props.busy),
    props.text)

def primaryButton(text: String, onClick: Callback): Node = actionButton(ButtonProps(text, onClick))
def dangerButton(text: String, onClick: Callback): Node = actionButton(ButtonProps(text, onClick, ButtonTone.Danger))
'''
    if w == "Badge":
        return f'''enum Tone derives Enumerated, Eq, Show:
  case Neutral, Success, Warning, Failure, Info

final case class BadgeProps(text: String, tone: Tone, dot: Boolean = false)
val badge: FC[BadgeProps] = FC[BadgeProps]: props =>
  val tone = props.tone match
    case Tone.Neutral => {ctx.tw()}
    case Tone.Success => {ctx.tw()}
    case Tone.Warning => {ctx.tw()}
    case Tone.Failure => {ctx.tw()}
    case Tone.Info => {ctx.tw()}
  span({c()}, cls :=? Some(tone), span({c()}).when(props.dot), props.text)

def toneOf(ok: Boolean): Tone = if ok then Tone.Success else Tone.Failure
'''
    if w == "Tabs":
        return f'''final case class TabsProps[T](tabs: List[T], selected: T, onSelect: T => Callback, label: T => String)
def tabs[T](props: TabsProps[T])(using Eq[T]): Node =
  Tags.nav({c()}, role := "tablist",
    props.tabs.toKeyedNodes(props.label)(tab =>
      Tags.button({c(2)}, cls :=? Option.when(tab === props.selected)({ctx.tw()}), role := "tab", aria.pressed := (tab === props.selected),
        onClick --> props.onSelect(tab), props.label(tab))))

def enumTabs[T](values: List[T], selected: T, onSelect: T => Callback): Node =
  tabs(TabsProps(values, selected, onSelect, t => t.toString.toLowerCase))(using Eq.fromUniversalEquals)
'''
    if w == "Modal":
        return f'''final case class ModalProps(title: String, open: Boolean, onClose: Callback, footer: Option[Node] = None)
val modal: FCOverChildren[ModalProps] = FC.withChildren[ModalProps]: (props, children) =>
  if !props.open then EmptyVdom
  else
    div({c()}, role := "dialog",
      div({c()}, onClick --> props.onClose),
      div({c(2)},
        headerTag({c()}, h2({c()}, props.title), Tags.button({c()}, onClick --> props.onClose, "close")),
        section({c()}, children.toMod),
        props.footer.ifDefined(f => footer({c()}, f))))
'''
    if w == "Dropdown":
        return f'''final case class DropdownProps[T](items: List[T], selected: Option[T], onSelect: Option[T] => Callback, label: T => String, placeholder: String = "choose")
def dropdown[T](props: DropdownProps[T]): Node =
  val (open, setOpen) = useState(false)
  div({c()},
    Tags.button({c()}, onClick --> setOpen(!open), props.selected.map(props.label).getOrElse(props.placeholder)),
    fragmentGate(open)(
      ul({c(2)},
        li({c()}, onClick --> (props.onSelect(None) *> setOpen(false)), props.placeholder),
        props.items.toKeyedNodes(props.label)(item =>
          li({c()}, cls :=? Option.when(props.selected.contains(item))({ctx.tw()}), onClick --> (props.onSelect(Some(item)) *> setOpen(false)), props.label(item))))))

def enumDropdown[T](selected: Option[T], onSelect: Option[T] => Callback)(using e: Enumerated[T]): Node =
  dropdown(DropdownProps(e.valueList, selected, onSelect, t => t.entryName))
'''
    if w == "SearchBox":
        return f'''final case class SearchFieldProps(value: String, onChange: String => Callback, placeholder: String = "search", onClear: Option[Callback] = None)
val searchBox: FC[SearchFieldProps] = FC[SearchFieldProps]: props =>
  val (focused, setFocused) = useState(false)
  label({c()}, cls :=? Option.when(focused)({ctx.tw()}),
    input.text({c()}, value := props.value, placeholder := props.placeholder, onChange ===> props.onChange, onFocus --> setFocused(true), onBlur --> setFocused(false)),
    props.onClear.ifDefined(clear => Tags.button({c()}, onClick --> clear, "clear").when(props.value.nonEmpty)))
'''
    if w == "Pagination":
        return f'''final case class PaginationProps(page: Int, pageSize: Int, total: Int, onPage: Int => Callback)
val pagination: FC[PaginationProps] = FC[PaginationProps]: props =>
  val pages = if props.total == 0 then 1 else (props.total + props.pageSize - 1) / props.pageSize
  Tags.nav({c()},
    Tags.button({c()}, disabled := (props.page <= 0), onClick --> props.onPage(props.page - 1), "previous"),
    span({c()}, {ctx.show_text("page ${props.page + 1} of $pages")}),
    Tags.button({c()}, disabled := (props.page + 1 >= pages), onClick --> props.onPage(props.page + 1), "next"))

def paged[A](items: List[A], page: Int, pageSize: Int): List[A] = items.drop(page * pageSize).take(pageSize)
'''
    if w == "Table":
        return f'''final case class Column[A](title: String, render: A => Node, width: Option[Int] = None, numeric: Boolean = false)
final case class GridProps[A](columns: List[Column[A]], rows: List[A], key: A => String, onRow: Option[A => Callback] = None, empty: String = "no rows")
def dataTable[A](props: GridProps[A]): Node =
  if props.rows.isEmpty then emptyState(EmptyStateProps(props.empty))
  else
    Tags.table({c()},
      thead({c()}, tr(props.columns.toNodes(col => th({c()}, cls :=? Option.when(col.numeric)({ctx.tw()}), col.width.ifDefined(w => Styles.width := w.px), col.title)))),
      tbody({c()},
        props.rows.toKeyedNodes(props.key)(row =>
          tr({c()}, props.onRow.ifDefined(f => onClick --> f(row)),
            props.columns.toNodes(col => td({c()}, cls :=? Option.when(col.numeric)({ctx.tw()}), col.render(row)))))))
'''
    if w == "EmptyState":
        return f'''final case class EmptyStateProps(text: String, action: Option[Node] = None)
val emptyState: FC[EmptyStateProps] = FC[EmptyStateProps]: props =>
  div({c()}, p({c()}, props.text), props.action.ifDefined(identity))
'''
    if w == "KeyValue":
        return f'''final case class KeyValueProps(pairs: List[(String, Node)], compact: Boolean = false)
val keyValue: FC[KeyValueProps] = FC[KeyValueProps]: props =>
  Tags.table({c()}, cls :=? Option.when(props.compact)({ctx.tw()}),
    tbody(props.pairs.toKeyedNodes(_._1)((k, v) => tr({c()}, th({c()}, k), td({c()}, v)))))

def pair(key: String, value: String): (String, Node) = (key, value)
'''
    if w == "Toasts":
        return f'''val toasts: FCN = FCN:
  val items = useStore(Environment.store)(_.toasts)
  div({c()}, aria.label := "toasts",
    items.zipWithIndex.toKeyedNodes((t, i) => s"$i-$t")((t, i) => div({c()}, cls :=? Option.when(i == items.length - 1)({ctx.tw()}), t)))
'''
    if w == "PageShell":
        return f'''final case class PageShellProps(title: String, actions: Actions, crumbs: List[String] = Nil, wide: Boolean = false)
val pageShell: FCOverChildren[PageShellProps] = FC.withChildren[PageShellProps]: (props, children) =>
  val busy = useStore(Environment.store)(_.busy)
  mainTag({c()}, cls :=? Option.when(props.wide)({ctx.tw()}),
    headerTag({c()}, breadcrumbs(BreadcrumbsProps(props.crumbs)), h1({c()}, props.title), spinner(SpinnerProps(busy))),
    section({c()}, children.toMod),
    toasts())
'''
    if w == "Breadcrumbs":
        return f'''final case class BreadcrumbsProps(crumbs: List[String])
val breadcrumbs: FC[BreadcrumbsProps] = FC[BreadcrumbsProps]: props =>
  Tags.nav({c()}, props.crumbs.zipWithIndex.toKeyedNodes(_._1)((crumb, i) =>
    span({c()}, cls :=? Option.when(i == props.crumbs.length - 1)({ctx.tw()}), crumb, span({c()}, "/").when(i < props.crumbs.length - 1))))
'''
    if w == "Spinner":
        return f'''final case class SpinnerProps(visible: Boolean, small: Boolean = false)
val spinner: FC[SpinnerProps] = FC[SpinnerProps]: props =>
  if props.visible then span({c()}, cls :=? Option.when(props.small)({ctx.tw()}), aria.busy := true, "...") else EmptyVdom
'''
    if w == "DateLabel":
        return f'''final case class DateLabelProps(at: Instant, relativeTo: Option[Instant] = None)
val dateLabel: FC[DateLabelProps] = FC[DateLabelProps]: props =>
  val text = props.relativeTo match
    case Some(now) if now.isAfter(props.at) => {ctx.show_text("${Duration.between(props.at, now).toMinutes} minutes ago")}
    case Some(_) => "upcoming"
    case None => props.at.toLocalDate.toString
  Tags.custom("time")({c()}, title := props.at.toString, text)

def dayLabel(day: LocalDate): Node = span({c()}, {ctx.show_text("${day.getDayOfWeek} $day")})
'''
    if w == "AmountLabel":
        return f'''final case class AmountLabelProps(amount: Amount, unit: String = "credits", emphasis: Boolean = false)
val amountLabel: FC[AmountLabelProps] = FC[AmountLabelProps]: props =>
  span({c()}, cls :=? Option.when(props.emphasis)({ctx.tw()}), cls :=? Option.when(props.amount.isNegative)({ctx.tw()}),
    props.amount.render, small({c()}, props.unit))

def total(amounts: List[Amount]): Amount = amounts.foldLeft[Amount](Amount.zero)(_ + _)
'''
    if w in ("TextField", "NumberField"):
        kind = {"TextField": "String", "NumberField": "Int"}[w]
        conv = {"String": "v => props.onChange(v)", "Int": "v => v.toIntOption.fold(Eff.unit)(props.onChange)"}[kind]
        tag = "number" if kind == "Int" else "text"
        return f'''final case class {w}Props(label: String, value: {kind}, onChange: {kind} => Callback, error: Option[Fault] = None, hint: Option[String] = None, required: Boolean = false)
val {name}: FC[{w}Props] = FC[{w}Props]: props =>
  val id = useId()
  label({c()}, cls :=? Option.when(props.error.isDefined)({ctx.tw()}), htmlFor := id,
    span({c()}, props.label, span({c()}, "*").when(props.required)),
    input.{tag}({c()}, Attrs.id := id, value := props.value.toString, Attrs.required := props.required, onChange ===> ({conv})),
    props.hint.ifDefined(h => small({c()}, h)),
    props.error.ifDefined(e => small({c()}, e.render)))
'''
    if w == "Toggle":
        return f'''final case class ToggleProps(on: Boolean, onToggle: Boolean => Callback, label: String)
val toggle: FC[ToggleProps] = FC[ToggleProps]: props =>
  label({c()}, cls :=? Option.when(props.on)({ctx.tw()}),
    input.checkbox({c()}, checked := props.on, onChange --> props.onToggle(!props.on)), props.label)
'''
    if w == "Stat":
        return f'''final case class StatProps(label: String, value: Long, delta: Option[Long] = None, unit: Option[String] = None)
val stat: FC[StatProps] = FC[StatProps]: props =>
  val delta = props.delta match
    case Some(d) if d > 0 => span({c()}, {ctx.show_text("+$d")})
    case Some(d) if d < 0 => span({c()}, {ctx.show_text("$d")})
    case _ => EmptyVdom
  div({c()}, small({c()}, props.label), strong({c()}, props.value.toString), props.unit.ifDefinedNode(u => small(u)), delta)

def statsRow(items: List[StatProps]): Node = div({c()}, items.toKeyedNodes(_.label)(s => stat(s)))
'''
    if w == "Chip":
        return f'''final case class ChipProps(text: String, active: Boolean, count: Int, onRemove: Option[Callback])
val chip: FC[ChipProps] = FC[ChipProps]: props =>
  span({c()}, cls :=? Option.when(props.active)({ctx.tw()}), props.text, small({c()}, props.count.toString).when(props.count > 1),
    props.onRemove.ifDefined(r => Tags.button({c()}, onClick --> r, "x")))
'''
    # every other widget: a props case class, a hook, a match and a few sites
    n = rng.randint(2, 4)
    fields = ", ".join(f"{f}: {t}" for f, t in [("text", "String"), ("active", "Boolean"), ("count", "Int"), ("note", "Option[String]")][:n])
    extra = f"\nobject {w}Props:\n  given Reusability[{w}Props] = Reusability.by_==\n" if rng.chance(0.4) else ""
    inner = [f"    span({c()}, props.text)"]
    if n >= 2:
        inner.append(f"    span({c()}).when(props.active)")
    if n >= 3:
        inner.append(f"    small({c()}, props.count.toString).when(props.count > 0)")
    if n >= 4:
        inner.append(f"    props.note.ifDefined(nn => small({c()}, nn))")
    tag = rng.pick(["div", "span", "section", "aside", "li", "article"])
    level = rng.pick(["low", "high"])
    scrutinee = "props.count" if n >= 3 else "props.text.length"
    return f'''final case class {w}Props({fields})
{extra}
val {name}: FC[{w}Props] = FC[{w}Props]: props =>
  val (hovered, setHovered) = useState(false)
  val level = {scrutinee} match
    case 0 => "{level}"
    case n if n < 10 => "some"
    case _ => "many"
  {tag}({c()}, cls :=? Option.when(hovered)({ctx.tw()}), data("level") := level, onMouseEnter --> setHovered(true), onMouseLeave --> setHovered(false),
{",\n".join(inner)})
'''


# ---------------------------------------------------------------------------------------------
# per-area components and pages

def field_node(ctx, name, t, obj="props.item"):
    expr = f"{obj}.{name}"
    c = ctx.classes
    if t.kind == "enum":
        return f"span({c()}, {expr}.entryName)" if t.text else f"span({c()}, {expr}.toString)"
    if t.kind == "id":
        return f"code({c()}, {expr}.raw.toString)"
    if t.expr == "Amount":
        return f"amountLabel(AmountLabelProps({expr}))"
    if t.expr == "Instant":
        return f"dateLabel(DateLabelProps({expr}))"
    if t.expr == "LocalDate":
        return f"dayLabel({expr})"
    if t.expr == "Boolean":
        return f"badge(BadgeProps(if {expr} then \"yes\" else \"no\", toneOf({expr})))"
    if t.kind == "option":
        return f"{expr}.ifDefinedNode(v => span({c()}, v.toString))"
    if t.kind == "list":
        return f"small({c()}, {ctx.show_text('${' + expr + '.length} items')})"
    if t.kind in ("map", "set", "several"):
        return f"small({c()}, {expr}.size.toString)"
    if t.expr == "String":
        return f"span({c()}, {expr})"
    if t.expr in ("Int", "Long"):
        return f"strong({c()}, {expr}.toString)"
    return f"span({c()}, {expr}.toString)"


def imports_for(area):
    return f"import meridian.frontend.component.widget.*\nimport meridian.frontend.component.{area.lower()}.*\nimport meridian.frontend.main.actions.*\nimport meridian.frontend.main.state.*"


def write_area_components(ctx, area, decls, entities):
    rng = ctx.rng
    pkg = f"component.{area.lower()}"
    entity = entities[0] if entities else None
    enums = [d for d in decls if d.kind == "enum"]
    others = [d for d in decls if d.kind == "case" and d is not entity]
    c = ctx.classes
    widgets = "import meridian.frontend.component.widget.*"
    if entity is None:
        ctx.write(pkg, f"{area}Panel", f'''final case class {area}PanelProps(title: String, notes: List[String], onAdd: String => Callback)
val {lower(area)}Panel: FC[{area}PanelProps] = FC[{area}PanelProps]: props =>
  val (text, setText) = useState("")
  section({c()},
    h2({c()}, props.title),
    ul({c()}, props.notes.zipWithIndex.toKeyedNodes(_._2.toString)((n, _) => li({c()}, n))),
    searchBox(SearchFieldProps(text, setText, "add a note")),
    primaryButton("add", props.onAdd(text) *> setText("")))
''', extra=widgets)
        return
    en = entity.name
    idn = entity.entity_id.name
    shown = [(n, t) for n, t in entity.fields if n != "id"][:6]
    status = next(((n, t) for n, t in entity.fields if t.kind == "enum"), None)
    enum_decl = next((d for d in decls if status and d.name == status[1].expr), None)

    ctx.flag = "active"
    body = [f"final case class {en}CardProps(item: {en}, selected: Boolean, onSelect: {idn} => Callback, onEdit: Option[{en} => Callback] = None)",
            f"object {en}CardProps:", f"  given Reusability[{en}CardProps] = Reusability.by(p => (p.item.id.raw, p.selected))", "",
            f"val {lower(en)}Card: FC[{en}CardProps] = FC[{en}CardProps]: props =>",
            "  val (expanded, setExpanded) = useState(false)",
            "  val active = props.selected || expanded"]
    if status and enum_decl:
        body.append(f"  val tone = props.item.{status[0]} match")
        for i, case in enumerate(enum_decl.cases[:5]):
            body.append(f"    case {enum_decl.name}.{case} => Tone.{['Success', 'Warning', 'Failure', 'Info', 'Neutral'][i % 5]}")
        if len(enum_decl.cases) > 5:
            body.append("    case _ => Tone.Neutral")
    body.append(f"  article({c()}, cls :=? Option.when(active)({ctx.tw()}), onClick --> props.onSelect(props.item.id),")
    badge = f", badge(BadgeProps(props.item.{status[0]}.entryName, tone))" if status and enum_decl and status[1].text else ""
    body.append(f"    headerTag({c()}, h3({c()}, {ctx.show_text('${props.item.id.raw}')}){badge}),")
    for n, t in shown[:4]:
        body.append(f"    div({c()}, small({c()}, \"{n}\"), {field_node(ctx, n, t)}),")
    body.append("    fragmentGate(expanded)(")
    for n, t in shown[4:6]:
        body.append(f"      div({c()}, small(\"{n}\"), {field_node(ctx, n, t)}),")
    body.append(f"      props.onEdit.ifDefinedNode(edit => Tags.button({c()}, onClick --> edit(props.item), \"edit\"))),")
    body.append(f"    Tags.button({c()}, onClick --> setExpanded(!expanded), if expanded then \"less\" else \"more\"))")
    ctx.write(pkg, f"{en}Card", "\n".join(body) + "\n", extra=widgets)
    ctx.flag = None

    cols = []
    for n, t in shown[:5]:
        numeric = ", numeric = true" if t.expr in ("Int", "Long", "Amount") else ""
        cols.append(f'    Column[{en}]("{n}", item => {field_node(ctx, n, t, "item")}{numeric})')
    body = [f"final case class {en}TableProps(items: List[{en}], onRow: {en} => Callback, sortBy: Option[String] = None)",
            f"val {lower(en)}Table: FC[{en}TableProps] = FC[{en}TableProps]: props =>",
            "  val rows = props.sortBy match", "    case Some(\"id\") => props.items.sortBy(_.id.raw)"]
    for n, t in shown[:2]:
        if t.order and t.kind == "prim":
            body.append(f"    case Some(\"{n}\") => props.items.sortBy(_.{n})")
    body += ["    case _ => props.items", f"  dataTable(GridProps[{en}](List(", ",\n".join(cols) + "),", "    rows, _.id.raw.toString, Some(props.onRow)))"]
    ctx.write(pkg, f"{en}Table", "\n".join(body) + "\n", extra=widgets)

    editable = [(n, t) for n, t in entity.fields if n != "id" and (t.expr in ("String", "Int", "Boolean") or (t.kind == "enum" and t.text))][:5]
    body = [f"final case class {en}EditorProps(draft: {en}, errors: List[Fault], onChange: {en} => Callback, onSave: Callback, onCancel: Callback)",
            f"val {lower(en)}Editor: FC[{en}EditorProps] = FC[{en}EditorProps]: props =>",
            "  def faultOf(field: String): Option[Fault] = props.errors.find(_.path == field)",
            f"  form({c()}, onSubmit ==> (e => e.preventDefaultIO *> props.onSave),"]
    for n, t in editable:
        if t.expr == "String":
            body.append(f'    textField(TextFieldProps("{n}", props.draft.{n}, v => props.onChange(props.draft.copy({n} = v)), faultOf("{n}"))),')
        elif t.expr == "Int":
            body.append(f'    numberField(NumberFieldProps("{n}", props.draft.{n}, v => props.onChange(props.draft.copy({n} = v)), faultOf("{n}"))),')
        elif t.expr == "Boolean":
            body.append(f'    toggle(ToggleProps(props.draft.{n}, v => props.onChange(props.draft.copy({n} = v)), "{n}")),')
        else:
            body.append(f'    enumDropdown[{t.expr}](Some(props.draft.{n}), v => v.fold(Eff.unit)(x => props.onChange(props.draft.copy({n} = x)))),')
    body.append(f"    footer({c()}, primaryButton(\"save\", props.onSave), dangerButton(\"cancel\", props.onCancel)))")
    ctx.write(pkg, f"{en}Editor", "\n".join(body) + "\n", extra=widgets)

    if enums:
        e0 = enums[0]
        body = [f"final case class {area}FiltersProps(filter: Option[{e0.name}], search: String, onFilter: Option[{e0.name}] => Callback, onSearch: String => Callback, count: Int)",
                f"val {lower(area)}Filters: FC[{area}FiltersProps] = FC[{area}FiltersProps]: props =>", "  val summary = props.filter match"]
        for case in e0.cases[:3]:
            body.append(f"    case Some({e0.name}.{case}) => \"{case.lower()} only\"")
        if len(e0.cases) > 3:
            body.append(f"    case Some(other) => {ctx.show_text('${other.entryName}')}" if e0.has("Enumerated") else "    case Some(other) => other.toString")
        body.append(f"    case None => {ctx.show_text('all ${props.count}')}")
        body.append(f"  div({c()},")
        body.append("    searchBox(SearchFieldProps(props.search, props.onSearch)),")
        if e0.has("Enumerated"):
            body.append(f"    enumDropdown[{e0.name}](props.filter, props.onFilter),")
        body.append(f"    span({c()}, summary))")
        ctx.write(pkg, f"{area}Filters", "\n".join(body) + "\n", extra=widgets)

    if others:
        o = others[0]
        pairs = [(n, t) for n, t in o.fields if t.kind == "prim"][:4]
        body = [f"final case class {o.name}SummaryProps(value: {o.name}, compact: Boolean = false)",
                f"val {lower(o.name)}Summary: FC[{o.name}SummaryProps] = FC[{o.name}SummaryProps]: props =>", "  keyValue(KeyValueProps(List("]
        body.append(",\n".join(f'    pair("{n}", props.value.{n}.toString)' for n, t in pairs) if pairs else f'    pair("kind", "{o.name}")')
        body.append("  ), props.compact))")
        body.append("")
        body.append(f"val {lower(o.name)}Chips: FC[List[{o.name}]] = FC[List[{o.name}]]: items =>")
        body.append(f"  div({c()}, items.zipWithIndex.toKeyedNodes(_._2.toString)((item, i) => chip(ChipProps({ctx.show_text('${i + 1}')}, i == 0, items.length, None))))")
        ctx.write(pkg, f"{o.name}Summary", "\n".join(body) + "\n", extra=widgets)


def write_area_pages(ctx, area, decls, entities):
    pkg = f"page.{area.lower()}"
    entity = entities[0] if entities else None
    enums = [d for d in decls if d.kind == "enum"]
    c = ctx.classes
    L = f"{area}Lenses"
    A = f"{area}Actions"
    if entity is None:
        ctx.write(pkg, f"{area}Page", f'''val {lower(area)}Page: FC[Actions] = FC[Actions]: actions =>
  val notes = useStore(Environment.store)(_.{lower(area)}.notes)
  pageShell(PageShellProps("{area}", actions, List("home", "{area.lower()}")))(
    {lower(area)}Panel({area}PanelProps("{area}", notes, text => actions.run({A}.note(text)))))
''', extra=imports_for(area))
        ctx.pages.append((area, f"{lower(area)}Page", None))
        return
    en = entity.name
    idn = entity.entity_id.name
    filters = (f"    {lower(area)}Filters({area}FiltersProps(slice.filter, slice.search, f => actions.run({A}.setFilter(f)), s => actions.run({A}.search(s)), filtered.length)),"
               if enums else f"    searchBox(SearchFieldProps(slice.search, s => actions.run({A}.search(s)))),")
    body = [f"val {lower(area)}ListPage: FC[Actions] = FC[Actions]: actions =>",
            f"  val slice = useStore(Environment.store)(_.{lower(area)})",
            "  val (view, setView) = useState(ListView.Cards)",
            f"  useEffect(slice.count == 0)(actions.run({A}.refresh).when(slice.count == 0))",
            "  val filtered = slice.visible.filter(item => slice.search.isEmpty || item.toString.toLowerCase.contains(slice.search.toLowerCase))",
            "  val shown = paged(filtered, slice.page, 12)",
            "  val body = view match",
            f"    case ListView.Cards => div({c()}, shown.toKeyedNodes(_.id.raw.toString)(item => {lower(en)}Card({en}CardProps(item, slice.selected.contains(item.id), id => actions.run({A}.select(Some(id))), Some(x => actions.run({A}.edit(x)))))))",
            f"    case ListView.Rows => {lower(en)}Table({en}TableProps(shown, item => actions.run({A}.select(Some(item.id)))))",
            f"    case ListView.Compact => ul({c()}, shown.toKeyedNodes(_.id.raw.toString)(item => li({c()}, item.id.raw.toString)))",
            f'  pageShell(PageShellProps("{area}", actions, List("home", "{area.lower()}"), wide = true))(',
            filters,
            "    enumTabs(ListView.values.toList, view, v => setView(v)),",
            "    body,",
            f"    pagination(PaginationProps(slice.page, 12, filtered.length, p => actions.run(Environment.store.setL({L}.page)(p)))),",
            f"    statsRow(List(StatProps(\"shown\", shown.length.toLong), StatProps(\"total\", slice.count.toLong), StatProps(\"selected\", slice.selected.size.toLong))))",
            "", "enum ListView:", "  case Cards, Rows, Compact"]
    ctx.write(pkg, f"{area}ListPage", "\n".join(body) + "\n", extra=imports_for(area))
    ctx.pages.append((area, f"{lower(area)}ListPage", None))

    detail = [f"final case class {area}DetailProps(id: {idn}, actions: Actions)",
              f"val {lower(area)}DetailPage: FC[{area}DetailProps] = FC[{area}DetailProps]: props =>",
              f"  val slice = useStore(Environment.store)(_.{lower(area)})",
              "  val item = slice.items.get(props.id)",
              "  val (confirm, setConfirm) = useState(false)",
              f"  useEffect(props.id.raw)(props.actions.run({A}.load(props.id)))",
              "  val body = item match",
              "    case None => spinner(SpinnerProps(true))",
              "    case Some(value) =>",
              f"      div({c()},",
              f"        {lower(en)}Card({en}CardProps(value, true, _ => Eff.unit)),",
              "        keyValue(KeyValueProps(List(pair(\"id\", value.id.raw.toString), pair(\"fields\", value.productArity.toString)))),",
              f"        Tags.button({c()}, onClick --> setConfirm(true), \"remove\"),",
              f"        modal(ModalProps(\"remove?\", confirm, setConfirm(false), Some(dangerButton(\"remove\", props.actions.run({A}.remove(props.id)) *> setConfirm(false)))))(p(\"this cannot be undone\")),",
              f"        slice.draft.ifDefinedNode(draft => {lower(en)}Editor({en}EditorProps(draft, slice.errors, d => props.actions.run({A}.edit(d)), props.actions.run({A}.save), props.actions.run({A}.discard)))))",
              f'  pageShell(PageShellProps({ctx.show_text("${props.id.raw}")}, props.actions, List("home", "{area.lower()}", "detail")))(',
              "    body,"]
    if ctx.flags.inline_fc:
        detail += ["    fc:", f"      small({c()}, {ctx.show_text('loaded ${slice.count} of ${slice.order.length}')}))"]
    else:
        detail += [f"    small({c()}, {ctx.show_text('loaded ${slice.count} of ${slice.order.length}')}))"]
    ctx.write(pkg, f"{area}DetailPage", "\n".join(detail) + "\n", extra=imports_for(area))
    ctx.pages.append((area, f"{lower(area)}DetailPage", idn))

    status = next(((n, t) for n, t in entity.fields if t.kind == "enum"), None)
    enum_decl = next((d for d in decls if status and d.name == status[1].expr), None)
    dash = [f"val {lower(area)}Dashboard: FC[Actions] = FC[Actions]: actions =>",
            f"  val slice = useStore(Environment.store)(_.{lower(area)})",
            "  val items = slice.items.values.toList"]
    if status and enum_decl:
        dash.append(f"  val byStatus = items.groupBy(_.{status[0]}).map((k, v) => (k, v.length))")
        dash.append(f"  def tone(k: {enum_decl.name}): Tone = k match")
        for i, case in enumerate(enum_decl.cases[:4]):
            dash.append(f"    case {enum_decl.name}.{case} => Tone.{['Success', 'Info', 'Warning', 'Failure'][i]}")
        if len(enum_decl.cases) > 4:
            dash.append("    case _ => Tone.Neutral")
    dash += [f'  pageShell(PageShellProps("{area} overview", actions, List("home", "{area.lower()}", "overview")))(',
             "    statsRow(List(StatProps(\"items\", items.length.toLong), StatProps(\"selected\", slice.selected.size.toLong, Some(1L)), StatProps(\"errors\", slice.errors.length.toLong))),"]
    if status and enum_decl:
        label = ctx.show_text("${k.entryName} $n") if enum_decl.has("Enumerated") else ctx.show_text("$k $n")
        dash.append(f"    div({c()}, byStatus.toList.sortBy(_._2).toKeyedNodes(_._1.toString)((k, n) => badge(BadgeProps({label}, tone(k))))),")
    dash.append(f"    primaryButton(\"refresh\", actions.run({A}.refresh)))")
    ctx.write(pkg, f"{area}Dashboard", "\n".join(dash) + "\n", extra=imports_for(area))
    ctx.pages.append((area, f"{lower(area)}Dashboard", None))


# ---------------------------------------------------------------------------------------------
# router, fake backend, main

def write_router(ctx):
    ids = {d.name: d for d in ctx.u.ids}
    lines = ["/** Every page of the frontend, and the paths that lead to them. */", "enum Page derives Eq:", "  case Home"]
    rules = []
    tour = ["Page.Home"]
    for area, name, idn in ctx.pages:
        case = name[0].upper() + name[1:]
        if idn is None:
            lines.append(f"  case {case}")
            rules.append(f'    Rule.static(Path.root / "{area.lower()}" / "{name.lower()}", Page.{case}),')
            tour.append(f"Page.{case}")
        else:
            lines.append(f"  case {case}(id: {idn})")
            is_long = ids[idn].raw_type == "Long"
            raw = "Path.long" if is_long else "Path.string"
            rules.append(f'    Rule.dynamic(Path.root / "{area.lower()}" / {raw})(id => Page.{case}({idn}(id))) {{ case Page.{case}(id) => id.value }},')
            tour.append(f"Page.{case}({idn}(7L))" if is_long else f'Page.{case}({idn}("k-7"))')
    lines += ["", "object Pages:", "  val rules: List[Rule[Page]] = List(", "    Rule.static(Path.root, Page.Home),"] + rules + ["  )", f"  val tour: List[Page] = List({', '.join(tour)})", ""]
    lines += ["object ConsoleRouter:", "  def render(actions: Actions)(router: Router[Page], page: Page): Node = page match", "    case Page.Home => homePage(actions)"]
    for area, name, idn in ctx.pages:
        case = name[0].upper() + name[1:]
        props = f"{area}EditProps" if name.endswith("EditPage") else f"{area}DetailProps"
        lines.append(f"    case Page.{case} => {name}(actions)" if idn is None else f"    case Page.{case}(id) => {name}({props}(id, actions))")
    lines += ["", "  def make(actions: Actions): Router[Page] = Router(Pages.rules, Page.Home)(render(actions))", "",
              "val homePage: FC[Actions] = FC[Actions]: actions =>",
              "  val events = useStore(Environment.store)(_.events)",
              '  pageShell(PageShellProps("home", actions))(',
              f"    ul({ctx.classes()}, events.take(10).zipWithIndex.toKeyedNodes(_._2.toString)((e, _) => li({ctx.classes()}, e))),",
              f"    small({ctx.classes()}, {ctx.show_text('${events.length} events')}))"]
    extra = "import meridian.frontend.component.widget.*\n" + "\n".join(f"import meridian.frontend.page.{area.lower()}.*" for area, _, _ in ctx.areas) + "\nimport meridian.frontend.main.state.*"
    ctx.write("router", "Pages", "\n".join(lines) + "\n", extra=extra)


def write_backend(ctx):
    rng = ctx.rng
    lines = ["/** Canned answers for every endpoint, from the samples: what the self-test's transport says. */", "object FakeBackend:",
             "  def respond(request: Request): Response = request.segments match"]
    dispatch = []
    for area, decls, entities in ctx.areas:
        items = ctx.by_area[area]
        if not items:
            continue
        dispatch.append(f'    case "v1" :: "{lower(area)}" :: rest => {area}Backend.respond(request.method, rest)')
        body = [f"object {area}Backend:", "  def respond(method: Method, rest: List[String]): Response = (method, rest) match"]
        seen = set()
        for e in items:
            pattern = ", ".join(f'"{v}"' if k == "fixed" else "_" for k, v in e.segments[2:])
            key = (e.method, pattern)
            if key in seen:
                continue
            seen.add(key)
            pat = f"(Method.{e.method.capitalize()}, List({pattern}))" if pattern else f"(Method.{e.method.capitalize()}, Nil)"
            if e.status is not None and e.output is None:
                body.append(f"    case {pat} => Response({e.status}, Nil, None)")
            elif e.output is not None:
                body.append(f"    case {pat} => Response(200, Nil, Some({e.output.sample(rng, 1)}.toJson))")
            else:
                body.append(f"    case {pat} => Response(200, Nil, None)")
        body.append('    case _ => Response(404, Nil, Some("\\"no such route\\""))')
        ctx.write("backend", f"{area}Backend", "\n".join(body) + "\n", extra="import meridian.model.samples.*")
    lines += dispatch + ['    case _ => Response(404, Nil, Some("\\"unknown area\\""))', "", "  val transport: Transport = Transport.of(respond)"]
    ctx.write("backend", "FakeBackend", "\n".join(lines) + "\n")


def write_main(ctx):
    checks = []
    for area, decls, entities in ctx.areas:
        for d in decls:
            if d.kind in ("case", "adt") and d.has("JsonCodec"):
                checks.append(f"    roundTrip[{d.name}](\"{d.name}\", {area}Samples.{d.sample_name()})")
            elif d.kind == "enum" and (d.has("JsonCodec") or d.has("Enumerated")):
                checks.append(f"    roundTrip[{d.name}](\"{d.name}\", {area}Samples.{d.sample_name()})")
    groups = [checks[i : i + 60] for i in range(0, len(checks), 60)]
    lines = ['''/** The self-test of the frontend: every page rendered to text with the fake backend behind
  * it, its handlers fired, every model round-tripped through its codec, and a checksum. */
object Main:
  private val out = Checksum.Builder()
  private var failures = 0

  def roundTrip[A](name: String, value: A)(using e: JsonEncoder[A], d: JsonDecoder[A]): Unit =
    val text = e.encodeJson(value)
    out.add(text)
    d.decodeJson(text) match
      case Right(back) if e.encodeJson(back) == text => ()
      case Right(_) =>
        failures += 1
        println(s"round trip changed $name")
      case Left(err) =>
        failures += 1
        println(s"round trip failed $name: $err")

  def main(args: Array[String]): Unit =
    val env: Env[ConsoleEnv] = Env(FakeBackend.transport).add(Session("token-1", Some("north"), "ada")).add(Clockwork.fixed(Instant.ofEpochSecond(1700000000L)))
    val actions = Actions(Runner(env))
    val router = ConsoleRouter.make(actions)
    Renderer.reset()
    var handlers = 0
    for page <- Pages.tour do
      router.set(page).runNow()
      val text = Renderer.mount(router.node)
      out.add(text)
      val paths = Renderer.handlerPaths
      handlers += paths.length
      for p <- paths.take(4) do
        val (element, event) = p.splitAt(p.indexOf('@'))
        Renderer.fire(element, event.drop(1), EventPayload("moon", true))
      val after = Renderer.mount(router.node)
      out.add(after)
      println(s"$page: ${text.length} chars, ${paths.length} handlers, ${after.length} after")
    println(Renderer.render(router.node).take(600))
    println(s"store writes ${Environment.store.writeCount}, renders ${Renderer.renders}, handlers $handlers")
    Log.drain().take(5).foreach(println)
''']
    lines += [f"    codecs{i}()" for i in range(len(groups))]
    lines.append('    println(s"failures $failures")')
    lines.append('    println(s"checksum ${out.value} over ${out.lines} lines")')
    for i, group in enumerate(groups):
        lines.append("")
        lines.append(f"  def codecs{i}(): Unit =")
        lines += group
    ctx.write("", "Main", "\n".join(lines) + "\n", extra="import meridian.frontend.router.*\nimport meridian.frontend.backend.*\nimport meridian.model.samples.*")
