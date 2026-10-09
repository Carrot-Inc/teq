"""The per-area files of the frontend: six components and three pages over one model area,
laid out one argument per line as the application's formatter would."""
import re


class Call:
    """A Scala call, rendered on one line when short and one argument per line otherwise."""

    def __init__(self, head, *args):
        self.head = head
        self.args = [a for a in args if a is not None]

    def render(self, indent=0):
        inline = [render(a, indent + 2) for a in self.args]
        one = f"{self.head}({', '.join(inline)})"
        if len(one) + indent <= 100 and len(inline) < 3 and not any("\n" in i for i in inline):
            return one
        pad = " " * (indent + 2)
        return f"{self.head}(\n" + ",\n".join(pad + i for i in inline) + ")"


def render(x, indent=0):
    return x.render(indent) if isinstance(x, Call) else x


def lower(name):
    return name[0].lower() + name[1:]


TONES = ["Success", "Warning", "Failure", "Info", "Neutral"]
ICONS = ["dot", "ring", "star", "flag", "bolt", "wave", "leaf", "gear"]
WIDGETS = "import meridian.frontend.component.widget.*"


def imports_for(area):
    return f"import meridian.frontend.component.widget.*\nimport meridian.frontend.component.{area.lower()}.*\nimport meridian.frontend.main.actions.*\nimport meridian.frontend.main.state.*"


def field_node(ctx, name, t, obj="props.item"):
    expr = f"{obj}.{name}"
    c = ctx.classes
    if t.kind == "enum":
        return Call("span", c(), f"{expr}.entryName" if t.text else f"{expr}.toString")
    if t.kind == "id":
        return Call("code", c(), f"{expr}.raw.toString")
    if t.expr == "Amount":
        return Call("amountLabel", f"AmountLabelProps({expr})")
    if t.expr == "Instant":
        return Call("dateLabel", f"DateLabelProps({expr})")
    if t.expr == "LocalDate":
        return f"dayLabel({expr})"
    if t.expr == "Boolean":
        return Call("badge", f'BadgeProps(if {expr} then "yes" else "no", toneOf({expr}))')
    if t.kind == "option":
        return f"{expr}.ifDefinedNode(v => {Call('span', c(), 'v.toString').render()})"
    if t.kind == "list":
        return Call("small", c(), ctx.show_text("${" + expr + ".length} items"))
    if t.kind in ("map", "set", "several"):
        return Call("small", c(), f"{expr}.size.toString")
    if t.expr == "String":
        return Call("span", c(), expr)
    if t.expr in ("Int", "Long"):
        return Call("strong", c(), f"{expr}.toString")
    return Call("span", c(), f"{expr}.toString")


def labelled_field(ctx, n, t, obj="props.item"):
    c = ctx.classes
    return Call("div", c(), Call("small", f'"{n}"'), field_node(ctx, n, t, obj))


def enum_match(decl, scrutinee, values, name, indent="  ", fallback=None):
    lines = [f"{indent}val {name} = {scrutinee} match"]
    for i, case in enumerate(decl.cases[:8]):
        lines.append(f"{indent}  case {decl.name}.{case} => {values[i % len(values)]}")
    if len(decl.cases) > 8:
        lines.append(f"{indent}  case _ => {fallback or values[-1]}")
    return lines


def status_of(decls, entity):
    status = next(((n, t) for n, t in entity.fields if t.kind == "enum"), None)
    decl = next((d for d in decls if status and d.name == status[1].expr), None)
    return status, decl


def write_area(ctx, area, decls, entities):
    entity = entities[0] if entities else None
    if entity is None:
        write_plain_area(ctx, area, decls)
        return
    write_card(ctx, area, decls, entity)
    write_table(ctx, area, decls, entity)
    write_editor(ctx, area, decls, entity)
    write_filters(ctx, area, decls, entity)
    write_row(ctx, area, decls, entity)
    write_bits(ctx, area, decls, entity)
    write_columns(ctx, area, decls, entity)
    write_helpers(ctx, area, decls, entity)
    write_list_page(ctx, area, decls, entity)
    write_detail_page(ctx, area, decls, entity)
    write_edit_page(ctx, area, decls, entity)
    write_dashboard(ctx, area, decls, entity)


# ---------------------------------------------------------------------------------------------

def write_plain_area(ctx, area, decls):
    c = ctx.classes
    pkg = f"component.{area.lower()}"
    ctx.flag = "open"
    body = Call("section", c(), Call("h2", c(), "props.title"),
                Call("ul", c(), "props.notes.zipWithIndex.toKeyedNodes(_._2.toString)((n, _) => " + Call("li", c(), "n").render() + ")"),
                Call("searchBox", 'SearchFieldProps(text, setText, "add a note")'),
                Call("primaryButton", '"add"', 'props.onAdd(text) *> setText("")'),
                Call("Tags.button", c(), "onClick --> setOpen(!open)", 'if open then "hide" else "show"'),
                Call("small", c(), ctx.show_text("${props.notes.length} notes")).render() + ".when(open)")
    ctx.write(pkg, f"{area}Panel", f'''final case class {area}PanelProps(title: String, notes: List[String], onAdd: String => Callback)
val {lower(area)}Panel: FC[{area}PanelProps] = FC[{area}PanelProps]: props =>
  val (text, setText) = useState("")
  val (open, setOpen) = useState(true)
  {body.render(2)}
''', extra=WIDGETS)
    ctx.flag = None
    ctx.write(f"page.{area.lower()}", f"{area}Page", f'''val {lower(area)}Page: FC[Actions] = FC[Actions]: actions =>
  val notes = useStore(Environment.store)(_.{lower(area)}.notes)
  {Call("pageShell", f'PageShellProps("{area}", actions, List("home", "{area.lower()}"))').render(2)}(
    {lower(area)}Panel({area}PanelProps("{area}", notes, text => actions.run({area}Actions.note(text)))))
''', extra=imports_for(area))
    ctx.pages.append((area, f"{lower(area)}Page", None))


def write_card(ctx, area, decls, entity):
    c = ctx.classes
    en = entity.name
    idn = entity.entity_id.name
    fields = [(n, t) for n, t in entity.fields if n != "id"]
    status, enum_decl = status_of(decls, entity)
    ctx.flag = "active"
    lines = [f"final case class {en}CardProps(item: {en}, selected: Boolean, onSelect: {idn} => Callback, onEdit: Option[{en} => Callback] = None, dense: Boolean = false)",
             f"object {en}CardProps:", f"  given Reusability[{en}CardProps] = Reusability.by(p => (p.item.id.raw, p.selected, p.dense))", "",
             f"val {lower(en)}Card: FC[{en}CardProps] = FC[{en}CardProps]: props =>",
             "  val (expanded, setExpanded) = useState(false)",
             "  val (hovered, setHovered) = useState(false)",
             "  val (tab, setTab) = useState(0)",
             "  val active = props.selected || expanded"]
    if status and enum_decl:
        lines += enum_match(enum_decl, f"props.item.{status[0]}", [f"Tone.{t}" for t in TONES], "tone")
        lines.append(f'  val icon = if props.selected then "{ICONS[0]}" else "{ICONS[1]}"')
    else:
        lines += ["  val tone = if props.selected then Tone.Info else Tone.Neutral", f'  val icon = if props.dense then "{ICONS[0]}" else "{ICONS[1]}"']
    lines += [f"  def fields = List({', '.join(chr(34) + n + chr(34) for n, _ in fields[:12])})",
              "  def labelOf(field: String): String = field.capitalize",
              "  def visible(field: String): Boolean = expanded || fields.indexOf(field) < 4",
              "  val density = if fields.length < 4 then \"compact\" else \"full\""]
    badge = Call("badge", f"BadgeProps(props.item.{status[0]}.entryName, tone)") if status and enum_decl and status[1].text else Call("badge", 'BadgeProps(density, tone)')
    header = Call("headerTag", c(), Call("span", c(), "icon"), Call("h3", c(), ctx.show_text("${props.item.id.raw}")), badge,
                  Call("small", c(), '"selected"').render() + ".when(props.selected)",
                  Call("small", c(), '"hover"').render() + ".when(hovered)")
    tabs = Call("Tags.nav", c(), *[Call("Tags.button", c(), f"cls :=? Option.when(tab == {i})({ctx.tw()})", f"onClick --> setTab({i})", f'"{name}"') for i, name in enumerate(["main", "details", "more"])])
    primary = Call("section", c(), *[labelled_field(ctx, n, t) for n, t in fields[:4]])
    details = Call("section", c(), *[labelled_field(ctx, n, t) for n, t in fields[4:9]]) if len(fields) > 4 else Call("section", c(), Call("small", c(), '"no details"'))
    more = Call("section", c(), *[labelled_field(ctx, n, t) for n, t in fields[9:14]]) if len(fields) > 9 else Call("section", c(), Call("small", c(), '"nothing more"'))
    footer = Call("footer", c(),
                  Call("Tags.button", c(), "onClick --> setExpanded(!expanded)", 'if expanded then "less" else "more"'),
                  "props.onEdit.ifDefined(edit => " + Call("Tags.button", c(), "onClick --> edit(props.item)", '"edit"').render() + ")",
                  Call("Tags.button", c(), "onClick --> props.onSelect(props.item.id)", '"select"').render() + ".unless(props.selected)",
                  Call("small", c(), ctx.show_text("${fields.length} fields")))
    body = Call("article", c(), f"cls :=? Option.when(active)({ctx.tw()})", f"cls :=? Option.when(props.dense)({ctx.tw()})", 'data("density") := density',
                "onMouseEnter --> setHovered(true)", "onMouseLeave --> setHovered(false)", "onClick --> props.onSelect(props.item.id)",
                header, tabs,
                "tab match\n      case 0 => " + primary.render(6) + "\n      case 1 => " + details.render(6) + "\n      case _ => " + more.render(6),
                "fragmentGate(expanded)(" + more.render(4) + ")",
                footer)
    lines.append("  " + body.render(2))
    ctx.write(f"component.{area.lower()}", f"{en}Card", "\n".join(lines) + "\n", extra=WIDGETS)
    ctx.flag = None


def write_table(ctx, area, decls, entity):
    c = ctx.classes
    en = entity.name
    idn = entity.entity_id.name
    fields = [(n, t) for n, t in entity.fields if n != "id"][:10]
    status, enum_decl = status_of(decls, entity)
    cols = []
    for n, t in fields:
        numeric = ", numeric = true" if t.expr in ("Int", "Long", "Amount") else ""
        width = f", width = Some({40 + 10 * (len(n) % 5)})" if t.expr in ("Boolean", "Int") else ""
        cols.append(Call(f'Column[{en}]', f'"{n}"', f"item => {render(field_node(ctx, n, t, 'item'))}" + numeric + width))
    lines = [f"final case class {en}TableProps(items: List[{en}], onRow: {en} => Callback, sortBy: Option[String] = None, descending: Boolean = false, selected: Option[{idn}] = None, onRemove: Option[{idn} => Callback] = None)",
             f"val {lower(en)}Table: FC[{en}TableProps] = FC[{en}TableProps]: props =>",
             "  val (sortKey, setSortKey) = useState(props.sortBy)",
             "  val sorted = sortKey match", "    case Some(\"id\") => props.items.sortBy(_.id.raw)"]
    for n, t in fields[:5]:
        if t.order and t.kind == "prim":
            lines.append(f"    case Some(\"{n}\") => props.items.sortBy(_.{n})")
    lines += ["    case Some(_) => props.items", "    case None => props.items",
              "  val rows = if props.descending then sorted.reverse else sorted",
              f"  val caption = {ctx.show_text('${rows.length} rows')}"]
    if status and enum_decl:
        lines.append(f"  def toneOfItem(item: {en}): Tone = item.{status[0]} match")
        for i, case in enumerate(enum_decl.cases[:8]):
            lines.append(f"    case {enum_decl.name}.{case} => Tone.{TONES[i % 5]}")
        if len(enum_decl.cases) > 8:
            lines.append("    case _ => Tone.Neutral")
        lines.append("  val tones = rows.map(toneOfItem)")
    actions = Call(f"Column[{en}]", '"actions"', "item => " + Call("span", c(),
                                                                    Call("Tags.button", c(), "onClick --> props.onRow(item)", '"open"'),
                                                                    "props.onRemove.ifDefined(remove => " + Call("Tags.button", c(), "onClick --> remove(item.id)", '"remove"').render() + ")").render(6))
    lines += ["  val idColumn = " + Call(f"Column[{en}]", '"id"', f"item => {Call('code', c(), 'item.id.raw.toString').render()}").render(2),
              "  val columns = (idColumn :: List(", "    " + ",\n    ".join(col.render(4) for col in cols) + ")) :+ " + actions.render(4),
              "  " + Call("div", c(),
                          Call("headerTag", c(), Call("span", c(), "caption"),
                               Call("Tags.button", c(), 'onClick --> setSortKey(Some("id"))', '"by id"'),
                               Call("Tags.button", c(), "onClick --> setSortKey(None)", '"unsorted"')),
                          Call("dataTable", Call(f"GridProps[{en}]", "columns", "rows", "_.id.raw.toString", "Some(props.onRow)", f'"no {area.lower()} yet"')),
                          Call("footer", c(), Call("small", c(), ctx.show_text("${rows.length} of ${props.items.length}")),
                               Call("small", c(), 'props.selected.map(s => s.raw.toString).getOrElse("none selected")'))).render(2)]
    ctx.write(f"component.{area.lower()}", f"{en}Table", "\n".join(lines) + "\n", extra=WIDGETS)


def write_editor(ctx, area, decls, entity):
    c = ctx.classes
    en = entity.name
    editable = [(n, t) for n, t in entity.fields if n != "id" and (t.expr in ("String", "Int", "Boolean") or (t.kind == "enum" and t.text))][:12]
    readonly = [(n, t) for n, t in entity.fields if n != "id" and (n, t) not in editable][:6]
    lines = [f"final case class {en}EditorProps(draft: {en}, errors: List[Fault], onChange: {en} => Callback, onSave: Callback, onCancel: Callback, busy: Boolean = false)",
             f"val {lower(en)}Editor: FC[{en}EditorProps] = FC[{en}EditorProps]: props =>",
             "  val (touched, setTouched) = useState(Set.empty[String])",
             "  val (preview, setPreview) = useState(false)",
             "  def faultOf(field: String): Option[Fault] = props.errors.find(_.path == field)",
             "  def touch(field: String): Callback = setTouched(touched + field)",
             "  val checks: Check[Unit] = Check.all(List("]
    checks = []
    for n, t in editable:
        if t.expr == "String":
            checks.append(f'    Rules.nonEmpty("{n}", props.draft.{n}).void')
        elif t.expr == "Int":
            checks.append(f'    Rules.nonNegative("{n}", props.draft.{n}.toLong).void')
    lines.append(",\n".join(checks) if checks else "    Check.valid(())")
    lines.append("  )).void")
    lines += ["  val state = (props.busy, checks.isValid, touched.isEmpty) match",
              "    case (true, _, _) => \"saving\"", "    case (_, false, _) => \"invalid\"", "    case (_, _, true) => \"untouched\"", "    case _ => \"ready\""]
    inputs = []
    for n, t in editable:
        if t.expr == "String":
            inputs.append(Call("textField", Call("TextFieldProps", f'"{n}"', f"props.draft.{n}", f"v => touch(\"{n}\") *> props.onChange(props.draft.copy({n} = v))", f'faultOf("{n}")', f'hint = Option.when(touched.contains("{n}"))("edited")')))
        elif t.expr == "Int":
            inputs.append(Call("numberField", Call("NumberFieldProps", f'"{n}"', f"props.draft.{n}", f"v => touch(\"{n}\") *> props.onChange(props.draft.copy({n} = v))", f'faultOf("{n}")')))
        elif t.expr == "Boolean":
            inputs.append(Call("toggle", Call("ToggleProps", f"props.draft.{n}", f"v => props.onChange(props.draft.copy({n} = v))", f'"{n}"')))
        else:
            inputs.append(Call(f"enumDropdown[{t.expr}]", f"Some(props.draft.{n})", f"v => v.fold(Eff.unit)(x => props.onChange(props.draft.copy({n} = x)))"))
    readonly_nodes = [labelled_field(ctx, n, t, "props.draft") for n, t in readonly]
    faults = "props.errors.toKeyedNodes(_.path)(f => " + Call("li", c(), "f.render").render() + ")"
    body = Call("form", c(), "onSubmit ==> (e => e.preventDefaultIO *> props.onSave)", 'data("state") := state',
                Call("fieldset", c(), Call("legend", c(), f'"{en}"'), *inputs),
                Call("fieldset", c(), Call("legend", c(), '"read only"'), *readonly_nodes) if readonly_nodes else None,
                Call("ul", c(), faults).render(4) + ".when(props.errors.nonEmpty)",
                "fragmentGate(preview)(" + Call("pre", c(), "props.draft.toString.take(200)").render(4) + ")",
                Call("footer", c(),
                     Call("actionButton", 'ButtonProps("save", props.onSave, disabled = checks.isInvalid, busy = props.busy)'),
                     Call("dangerButton", '"cancel"', "props.onCancel"),
                     Call("Tags.button", c(), "onClick --> setPreview(!preview)", '"preview"'),
                     Call("small", c(), ctx.show_text("${touched.size} touched, $state"))))
    lines.append("  " + body.render(2))
    ctx.write(f"component.{area.lower()}", f"{en}Editor", "\n".join(lines) + "\n", extra=WIDGETS)


def write_filters(ctx, area, decls, entity):
    c = ctx.classes
    enums = [d for d in decls if d.kind == "enum" and d.has("Enumerated")]
    bools = [n for n, t in entity.fields if t.expr == "Boolean"][:2]
    e0 = enums[0] if enums else None
    ftype = f"Option[{e0.name}]" if e0 else "Option[String]"
    lines = [f"final case class {area}FiltersProps(filter: {ftype}, search: String, onFilter: {ftype} => Callback, onSearch: String => Callback, count: Int, total: Int)",
             f"val {lower(area)}Filters: FC[{area}FiltersProps] = FC[{area}FiltersProps]: props =>",
             "  val (open, setOpen) = useState(false)",
             "  val (recent, setRecent) = useState(List.empty[String])",
             "  def remember(text: String): Callback = setRecent((text :: recent).distinct.take(5)) *> props.onSearch(text)"]
    if e0:
        lines.append("  val summary = props.filter match")
        for case in e0.cases[:4]:
            lines.append(f"    case Some({e0.name}.{case}) => \"{case.lower()} only\"")
        if len(e0.cases) > 4:
            lines.append(f"    case Some(other) => {ctx.show_text('${other.entryName}')}")
        lines.append(f"    case None => {ctx.show_text('all ${props.count} of ${props.total}')}")
    else:
        lines.append(f"  val summary = props.filter.map(f => {ctx.show_text('matching $f')}).getOrElse({ctx.show_text('all ${props.count}')})")
    lines += ["  val ratio = if props.total == 0 then 0 else props.count * 100 / props.total",
              "  val coverage = ratio match", "    case 100 => \"everything\"", "    case n if n > 50 => \"most\"", "    case 0 => \"nothing\"", "    case _ => \"some\""]
    ctx.flag = "open"
    parts = [Call("searchBox", "SearchFieldProps(props.search, remember, onClear = Some(props.onSearch(\"\")))")]
    if e0:
        parts.append(Call(f"enumDropdown[{e0.name}]", "props.filter", "props.onFilter"))
    for b in bools:
        parts.append(Call("toggle", f'ToggleProps(open, _ => setOpen(!open), "{b}")'))
    parts.append(Call("Tags.button", c(), "onClick --> setOpen(!open)", 'if open then "fewer" else "more"'))
    parts.append(Call("span", c(), "summary"))
    parts.append(Call("small", c(), "coverage"))
    parts.append("fragmentGate(open)(" + Call("div", c(), Call("small", c(), '"recent"'),
                                               Call("ul", c(), "recent.toKeyedNodes(identity)(r => " + Call("li", c(), "onClick --> props.onSearch(r)", "r").render() + ")"),
                                               Call("chip", 'ChipProps(summary, props.filter.isDefined, props.count, Some(props.onFilter(None)))')).render(4) + ")")
    lines.append("  " + Call("div", c(), *parts).render(2))
    ctx.write(f"component.{area.lower()}", f"{area}Filters", "\n".join(lines) + "\n", extra=WIDGETS)
    ctx.flag = None


def write_row(ctx, area, decls, entity):
    c = ctx.classes
    en = entity.name
    fields = [(n, t) for n, t in entity.fields if n != "id"][:4]
    status, enum_decl = status_of(decls, entity)
    lines = [f"final case class {en}RowProps(item: {en}, index: Int, onPick: {en} => Callback, picked: Boolean)",
             f"val {lower(en)}Row: FC[{en}RowProps] = memoBy(FC[{en}RowProps]: props =>"]
    if status and enum_decl:
        lines += enum_match(enum_decl, f"props.item.{status[0]}", [f"Tone.{t}" for t in TONES], "tone")
    else:
        lines.append("  val tone = if props.picked then Tone.Info else Tone.Neutral")
    lines.append('  val icon = if props.index % 2 == 0 then "even" else "odd"')
    ctx.flag = "props.picked"
    cells = [Call("span", c(), "props.index.toString"), Call("span", c(), "icon")] + [field_node(ctx, n, t) for n, t in fields]
    if status and enum_decl and status[1].text:
        cells.append(Call("badge", f"BadgeProps(props.item.{status[0]}.entryName, tone, dot = true)"))
    cells.append(Call("Tags.button", c(), "onClick --> props.onPick(props.item)", '"pick"').render() + ".unless(props.picked)")
    body = Call("li", c(), f"cls :=? Option.when(props.index % 2 == 0)({ctx.tw()})", f"cls :=? Option.when(props.picked)({ctx.tw()})", "onClick --> props.onPick(props.item)", *cells)
    lines.append("  " + body.render(2))
    lines.append(", p => (p.item.id.raw, p.index, p.picked))")
    ctx.flag = None
    lines += ["", f"final case class {en}RowsProps(items: List[{en}], onPick: {en} => Callback, picked: Option[{entity.entity_id.name}])",
              f"val {lower(en)}Rows: FC[{en}RowsProps] = FC[{en}RowsProps]: props =>",
              "  val (limit, setLimit) = useState(20)",
              "  val shown = props.items.take(limit)",
              "  " + Call("div", c(),
                          Call("headerTag", c(), Call("small", c(), ctx.show_text("${shown.length} of ${props.items.length}"))),
                          Call("ol", c(), f"shown.zipWithIndex.toKeyedNodes(_._1.id.raw.toString)((item, i) => {lower(en)}Row({en}RowProps(item, i, props.onPick, props.picked.contains(item.id))))"),
                          Call("footer", c(), Call("Tags.button", c(), "onClick --> setLimit(limit + 20)", '"more"').render() + ".when(limit < props.items.length)")).render(2)]
    ctx.write(f"component.{area.lower()}", f"{en}Row", "\n".join(lines) + "\n", extra=WIDGETS)


def write_columns(ctx, area, decls, entity):
    """The columns of the area's tables: one per field, named at every construction site."""
    en = entity.name
    fields = [(n, t) for n, t in entity.fields if n != "id"][:10]
    lines = [f"final case class {en}Column(", "  key: String,", "  title: String,", f"  render: {en} => String,",
             "  sortable: Boolean = false,", "  width: Option[Int] = None,", "  numeric: Boolean = false", ")", "",
             f"val {lower(en)}Columns: List[{en}Column] = List(",
             f"  {en}Column(key = \"id\", title = \"id\", render = _.id.raw.toString, sortable = true, width = Some(80)),"]
    rows = []
    for n, t in fields:
        words = " ".join(w.lower() for w in re.findall(r"[A-Z]?[a-z]+|[A-Z]+", n))
        args = [f"key = \"{n}\"", f"title = \"{words}\"", f"render = item => item.{n}.toString"]
        if t.expr in ("String", "Int", "Long", "Amount", "Instant", "LocalDate") or t.kind == "enum":
            args.append("sortable = true")
        if t.expr in ("Int", "Long", "Amount"):
            args.append("numeric = true")
        if t.expr in ("Boolean", "Int"):
            args.append("width = Some(64)")
        elif t.kind in ("list", "set"):
            args.append("width = Some(240)")
        rows.append(f"  {en}Column({', '.join(args)})")
    lines.append(",\n".join(rows))
    lines += [")", "",
              f"def {lower(en)}ColumnFor(key: String): Option[{en}Column] = {lower(en)}Columns.find(_.key == key)",
              f"def {lower(en)}Cells(item: {en}): List[String] = {lower(en)}Columns.map(_.render(item))",
              f"def {lower(en)}Header: List[String] = {lower(en)}Columns.map(_.title)",
              f"def {lower(en)}WidthOf(column: {en}Column): Int = column.width.getOrElse(column.title.length * 8 + 16)",
              f"def {lower(en)}SortedBy(items: List[{en}], key: String): List[{en}] = {lower(en)}ColumnFor(key).filter(_.sortable) match",
              "  case Some(column) => items.sortBy(column.render)",
              "  case None => items"]
    ctx.write(f"component.{area.lower()}", f"{en}Columns", "\n".join(lines) + "\n", extra=imports_for(area))


def write_bits(ctx, area, decls, entity):
    """The sidebar, the toolbar, the status badge and the summary of a related type, in one file."""
    c = ctx.classes
    en = entity.name
    status, enum_decl = status_of(decls, entity)
    others = [d for d in decls if d.kind == "case" and d is not entity]
    lines = [f"enum {area}Section derives Enumerated, Eq:", "  case Overview, Items, Detail, Settings, Activity, Exports", "",
             f"final case class {area}SidebarProps(section: {area}Section, count: Int, onSection: {area}Section => Callback)",
             f"val {lower(area)}Sidebar: FC[{area}SidebarProps] = FC[{area}SidebarProps]: props =>",
             "  val (collapsed, setCollapsed) = useState(false)",
             f"  def label(section: {area}Section): String = section match",
             f"    case {area}Section.Overview => \"overview\"",
             f"    case {area}Section.Items => {ctx.show_text('items (${props.count})')}",
             f"    case {area}Section.Detail => \"detail\"",
             f"    case {area}Section.Settings => \"settings\"",
             f"    case {area}Section.Activity => \"activity\"",
             f"    case {area}Section.Exports => \"exports\""]
    ctx.flag = "collapsed"
    item = Call("li", c(), f"cls :=? Option.when(section === props.section)({ctx.tw()})", "onClick --> props.onSection(section)", "label(section)")
    sidebar = Call("aside", c(), f"cls :=? Option.when(collapsed)({ctx.tw()})",
                   Call("Tags.button", c(), "onClick --> setCollapsed(!collapsed)", 'if collapsed then ">" else "<"'),
                   Call("ul", c(), f"Enumerated[{area}Section].valueList.toKeyedNodes(_.entryName)(section => {item.render(6)})"),
                   Call("footer", c(), Call("small", c(), f'"{area.lower()}"')).render(4) + ".unless(collapsed)")
    lines.append("  " + sidebar.render(2))
    ctx.flag = None
    lines += ["", f"enum {area}Mode:", "  case Browse, Select, Edit", "",
              f"final case class {area}ToolbarProps(mode: {area}Mode, onMode: {area}Mode => Callback, onRefresh: Callback, onCreate: Option[Callback], selectedCount: Int)",
              f"val {lower(area)}Toolbar: FC[{area}ToolbarProps] = FC[{area}ToolbarProps]: props =>",
              "  val (menu, setMenu) = useState(false)",
              "  val title = props.mode match",
              f"    case {area}Mode.Browse => \"browse\"",
              f"    case {area}Mode.Select => {ctx.show_text('${props.selectedCount} selected')}",
              f"    case {area}Mode.Edit => \"editing\""]
    ctx.flag = "menu"
    toolbar = Call("div", c(), "role := \"toolbar\"",
                   Call("strong", c(), "title"),
                   f"enumTabs({area}Mode.values.toList, props.mode, m => props.onMode(m))",
                   Call("primaryButton", '"refresh"', "props.onRefresh"),
                   "props.onCreate.ifDefined(create => " + Call("actionButton", 'ButtonProps("new", create, ButtonTone.Secondary)').render() + ")",
                   Call("Tags.button", c(), "onClick --> setMenu(!menu)", '"menu"'),
                   "fragmentGate(menu)(" + Call("ul", c(),
                                                 Call("li", c(), f"onClick --> (props.onMode({area}Mode.Select) *> setMenu(false))", '"select all"'),
                                                 Call("li", c(), f"onClick --> (props.onMode({area}Mode.Browse) *> setMenu(false))", '"clear"'),
                                                 Call("li", c(), "onClick --> setMenu(false)", '"close"')).render(4) + ")")
    lines.append("  " + toolbar.render(2))
    ctx.flag = None
    if status and enum_decl:
        lines += ["", f"final case class {en}BadgeProps(value: {enum_decl.name}, large: Boolean = false)",
                  f"val {lower(en)}Badge: FC[{en}BadgeProps] = FC[{en}BadgeProps]: props =>"]
        lines += enum_match(enum_decl, "props.value", [f"Tone.{t}" for t in TONES], "tone")
        lines.append("  " + Call("span", c(), f"cls :=? Option.when(props.large)({ctx.tw()})",
                                 Call("badge", f"BadgeProps({'props.value.entryName' if status[1].text else 'props.value.toString'}, tone, dot = props.large)")).render(2))
    if others:
        o = others[0]
        prims = [(n, t) for n, t in o.fields if t.kind == "prim"][:5]
        options = [(n, t) for n, t in o.fields if t.kind == "option"][:2]
        lines += ["", f"final case class {o.name}SummaryProps(value: {o.name}, compact: Boolean = false)",
                  f"val {lower(o.name)}Summary: FC[{o.name}SummaryProps] = FC[{o.name}SummaryProps]: props =>"]
        for n, t in options:
            lines += [f"  val {n}Text = props.value.{n} match", "    case Some(v) => v.toString", "    case None => \"none\""]
        pairs = [f'pair("{n}", props.value.{n}.toString)' for n, t in prims] + [f'pair("{n}", {n}Text)' for n, t in options]
        lines.append("  " + Call("div", c(), Call("keyValue", Call("KeyValueProps", "List(" + ", ".join(pairs) + ")" if pairs else f'List(pair("kind", "{o.name}"))', "props.compact")),
                                 Call("small", c(), f'"{o.name}"').render() + ".unless(props.compact)").render(2))
    ctx.write(f"component.{area.lower()}", f"{area}Bits", "\n".join(lines) + "\n", extra=WIDGETS)


def write_helpers(ctx, area, decls, entity):
    """Pure helpers over the area's types: labels, sorting, grouping, formatting."""
    en = entity.name
    idn = entity.entity_id.name
    status, enum_decl = status_of(decls, entity)
    texts = [n for n, t in entity.fields if t.expr == "String"][:3]
    numbers = [n for n, t in entity.fields if t.expr in ("Int", "Long")][:3]
    dates = [n for n, t in entity.fields if t.expr in ("Instant", "LocalDate")][:2]
    lines = [f"/** What the {area.lower()} pages compute from their items. */", f"object {area}Helpers:",
             f"  final case class Grouped(key: String, items: List[{en}], share: Int)",
             f"  final case class Ranked(item: {en}, rank: Int, score: Long)",
             "",
             f"  def label(item: {en}): String = " + (ctx.show_text("${item.id.raw} ${item." + texts[0] + "}") if texts else ctx.show_text("${item.id.raw}")),
             f"  def key(item: {en}): String = item.id.raw.toString",
             f"  def sortKeys: List[String] = List({', '.join(chr(34) + n + chr(34) for n in ['id'] + texts + numbers)})",
             f"  def byKey(items: List[{en}], key: String): List[{en}] = key match",
             "    case \"id\" => items.sortBy(_.id.raw)"]
    for n in texts:
        lines.append(f"    case \"{n}\" => items.sortBy(_.{n})")
    for n in numbers:
        lines.append(f"    case \"{n}\" => items.sortBy(_.{n})")
    lines.append("    case _ => items")
    lines += ["", f"  def groups(items: List[{en}]): List[Grouped] ="]
    if status and enum_decl:
        lines.append(f"    items.groupBy(_.{status[0]}.toString).toList.sortBy(_._1).map((k, v) => Grouped(k, v, if items.isEmpty then 0 else v.length * 100 / items.length))")
    else:
        lines.append("    items.groupBy(i => key(i).take(1)).toList.sortBy(_._1).map((k, v) => Grouped(k, v, if items.isEmpty then 0 else v.length * 100 / items.length))")
    lines += ["", f"  def score(item: {en}): Long = " + (" + ".join(f"item.{n}.toLong" for n in numbers) if numbers else "item.id.raw.toString.length.toLong"),
              f"  def ranked(items: List[{en}]): List[Ranked] = items.sortBy(i => -score(i)).zipWithIndex.map((i, r) => Ranked(i, r + 1, score(i)))",
              f"  def top(items: List[{en}], n: Int): List[{en}] = ranked(items).take(n).map(_.item)",
              "",
              f"  def summary(items: List[{en}]): String = items.length match",
              "    case 0 => \"none\"",
              "    case 1 => \"one\"",
              f"    case n => {ctx.show_text('$n items')}",
              "",
              f"  def matches(item: {en}, text: String): Boolean =",
              "    text.isEmpty || label(item).toLowerCase.contains(text.toLowerCase)",
              f"  def selectNext(order: List[{idn}], selected: Option[{idn}]): Option[{idn}] = selected match",
              "    case Some(id) => order.dropWhile(_ != id).drop(1).headOption.orElse(order.headOption)",
              "    case None => order.headOption",
              f"  def toggle(selected: Set[{idn}], id: {idn}): Set[{idn}] = if selected.contains(id) then selected - id else selected + id"]
    if dates:
        lines += ["", f"  def recent(items: List[{en}], limit: Int): List[{en}] = items.sortBy(_.{dates[0]}).reverse.take(limit)"]
    if status and enum_decl:
        lines += ["", f"  def describe(value: {enum_decl.name}): String = value match"]
        for case in enum_decl.cases[:6]:
            lines.append(f"    case {enum_decl.name}.{case} => \"{case.lower()}\"")
        if len(enum_decl.cases) > 6:
            lines.append("    case other => other.toString.toLowerCase")
    ctx.write(f"page.{area.lower()}", f"{area}Helpers", "\n".join(lines) + "\n", extra=WIDGETS)


def write_list_page(ctx, area, decls, entity):
    c = ctx.classes
    en = entity.name
    enums = [d for d in decls if d.kind == "enum" and d.has("Enumerated")]
    L = f"{area}Lenses"
    A = f"{area}Actions"
    filters = Call(f"{lower(area)}Filters", Call(f"{area}FiltersProps", "slice.filter", "slice.search", f"f => actions.run({A}.setFilter(f))", f"s => actions.run({A}.search(s))", "filtered.length", "slice.count")) if enums else Call("searchBox", f"SearchFieldProps(slice.search, s => actions.run({A}.search(s)))")
    lines = [f"val {lower(area)}ListPage: FC[Actions] = FC[Actions]: actions =>",
             f"  val slice = useStore(Environment.store)(_.{lower(area)})",
             "  val (view, setView) = useState(ListView.Cards)",
             f"  val (mode, setMode) = useState({area}Mode.Browse)",
             f"  val (activeSection, setSection) = useState({area}Section.Items)",
             "  val (bulk, setBulk) = useState(false)",
             f"  useEffect(slice.count == 0)(actions.run({A}.refresh).when(slice.count == 0))",
             "  val filtered = slice.visible.filter(item => slice.search.isEmpty || item.toString.toLowerCase.contains(slice.search.toLowerCase))",
             "  val shown = paged(filtered, slice.page, 12)",
             "  val body = view match",
             f"    case ListView.Cards => " + Call("div", c(), f"shown.toKeyedNodes(_.id.raw.toString)(item => {lower(en)}Card({en}CardProps(item, slice.selected.contains(item.id), id => actions.run({A}.select(Some(id))), Some(x => actions.run({A}.edit(x))), dense = mode == {area}Mode.Select)))").render(6),
             f"    case ListView.Rows => {lower(en)}Table({en}TableProps(shown, item => actions.run({A}.select(Some(item.id))), selected = slice.selected, onRemove = Some(id => actions.run({A}.remove(id)))))",
             f"    case ListView.Compact => {lower(en)}Rows({en}RowsProps(shown, item => actions.run({A}.select(Some(item.id))), slice.selected))",
             "  val empty = " + Call("emptyState", f'EmptyStateProps("no {area.lower()} match", Some(primaryButton("refresh", actions.run({A}.refresh))))').render(2),
             "  " + Call("pageShell", f'PageShellProps("{area}", actions, List("home", "{area.lower()}"), wide = true)').render(2) + "(",
             "    " + Call("div", c(),
                          Call(f"{lower(area)}Sidebar", f"{area}SidebarProps(activeSection, slice.count, s => setSection(s))"),
                          Call("section", c(),
                               Call(f"{lower(area)}Toolbar", f"{area}ToolbarProps(mode, m => setMode(m), actions.run({A}.refresh), None, slice.selected.fold(0)(_ => 1))"),
                               filters, "enumTabs(ListView.values.toList, view, v => setView(v))",
                               "if filtered.isEmpty then empty else body",
                               Call("pagination", f"PaginationProps(slice.page, 12, filtered.length, p => actions.run(Environment.store.setL({L}.page)(p)))"),
                               Call("statsRow", f'List(StatProps("shown", shown.length.toLong), StatProps("total", slice.count.toLong), StatProps("selected", slice.selected.size.toLong), StatProps("page", slice.page.toLong + 1))'),
                               Call("Tags.button", c(), "onClick --> setBulk(true)", '"bulk"'),
                               Call("modal", 'ModalProps("bulk actions", bulk, setBulk(false))').render() + "(" + Call("p", c(), ctx.show_text("apply to ${slice.selected.size} items")).render(8) + ")")).render(4) + ")",
             "", "enum ListView:", "  case Cards, Rows, Compact"]
    ctx.write(f"page.{area.lower()}", f"{area}ListPage", "\n".join(lines) + "\n", extra=imports_for(area))
    ctx.pages.append((area, f"{lower(area)}ListPage", None))


def write_detail_page(ctx, area, decls, entity):
    c = ctx.classes
    en = entity.name
    idn = entity.entity_id.name
    A = f"{area}Actions"
    status, enum_decl = status_of(decls, entity)
    others = [d for d in decls if d.kind == "case" and d is not entity]
    lines = [f"final case class {area}DetailProps(id: {idn}, actions: Actions)",
             f"val {lower(area)}DetailPage: FC[{area}DetailProps] = FC[{area}DetailProps]: props =>",
             f"  val slice = useStore(Environment.store)(_.{lower(area)})",
             "  val item = slice.items.get(props.id)",
             "  val (confirm, setConfirm) = useState(false)",
             f"  val (activeSection, setSection) = useState({area}Section.Detail)",
             f"  useEffect(props.id.raw)(props.actions.run({A}.load(props.id)))",
             "  val crumbs = List(\"home\", \"" + area.lower() + "\", props.id.raw.toString)",
             "  val heading = item match",
             f"    case Some(value) => {ctx.show_text('${value.id.raw}')}",
             f"    case None => {ctx.show_text('loading ${props.id.raw}')}"]
    sections = ["      " + Call("div", c(),
                                Call(f"{lower(en)}Card", f"{en}CardProps(value, true, _ => Eff.unit)"),
                                Call("keyValue", f'KeyValueProps({lower(en)}Columns.map(column => pair(column.title, column.render(value))) :+ pair("section", activeSection.entryName))'),
                                (Call(f"{lower(en)}Badge", f"{en}BadgeProps(value.{status[0]}, large = true)") if status and enum_decl else None),
                                Call("Tags.button", c(), "onClick --> setConfirm(true)", '"remove"'),
                                Call("modal", f'ModalProps("remove?", confirm, setConfirm(false), Some(dangerButton("remove", props.actions.run({A}.remove(props.id)) *> setConfirm(false))))').render() + '(p("this cannot be undone"))',
                                f"slice.draft.ifDefinedNode(draft => {lower(en)}Editor({en}EditorProps(draft, slice.errors, d => props.actions.run({A}.edit(d)), props.actions.run({A}.save), props.actions.run({A}.discard), slice.loading)))").render(6)]
    lines += ["  val body = item match", "    case None => spinner(SpinnerProps(true))", "    case Some(value) =>"] + sections
    lines += ["  val panel = activeSection match",
              f"    case {area}Section.Activity => " + Call("ul", c(), "slice.errors.toKeyedNodes(_.path)(f => " + Call("li", c(), "f.render").render() + ")").render(6),
              f"    case {area}Section.Settings => " + Call("keyValue", 'KeyValueProps(List(pair("page", slice.page.toString), pair("search", slice.search)))').render(6),
              "    case _ => body"]
    lines += ["  " + Call("pageShell", "PageShellProps(heading, props.actions, crumbs)").render(2) + "(",
              "    " + Call("div", c(), Call(f"{lower(area)}Sidebar", f"{area}SidebarProps(activeSection, slice.count, s => setSection(s))"), "panel").render(4) + ","]
    if ctx.flags.inline_fc:
        lines += ["    fc:", f"      small({c()}, {ctx.show_text('loaded ${slice.count} of ${slice.order.length}')}))"]
    else:
        lines += [f"    small({c()}, {ctx.show_text('loaded ${slice.count} of ${slice.order.length}')}))"]
    ctx.write(f"page.{area.lower()}", f"{area}DetailPage", "\n".join(lines) + "\n", extra=imports_for(area))
    ctx.pages.append((area, f"{lower(area)}DetailPage", idn))


def write_edit_page(ctx, area, decls, entity):
    c = ctx.classes
    en = entity.name
    idn = entity.entity_id.name
    A = f"{area}Actions"
    H = f"{area}Helpers"
    lines = [f"final case class {area}EditProps(id: {idn}, actions: Actions)",
             f"val {lower(area)}EditPage: FC[{area}EditProps] = FC[{area}EditProps]: props =>",
             f"  val slice = useStore(Environment.store)(_.{lower(area)})",
             "  val (step, setStep) = useState(EditStep.Fields)",
             "  val (dirty, setDirty) = useState(false)",
             f"  useEffect(props.id.raw)(props.actions.run({A}.load(props.id)))",
             "  val draft = slice.draft.orElse(slice.items.get(props.id))",
             "  val checks = draft.map(d => " + A + ".validate(d))",
             "  val stepLabel = step.toString.toLowerCase",
             "  val verdict = checks match",
             "    case Some(Check.Valid(_)) => \"valid\"",
             f"    case Some(Check.Invalid(faults)) => {ctx.show_text('${faults.length} faults')}",
             "    case None => \"nothing to check\"",
             "  val editor = draft.ifDefinedNode(d => " + Call(f"{lower(en)}Editor", Call(f"{en}EditorProps", "d", "slice.errors", f"x => setDirty(true) *> props.actions.run({A}.edit(x))", f"props.actions.run({A}.save) *> setStep(EditStep.Done)", f"props.actions.run({A}.discard) *> setStep(EditStep.Fields)", "slice.loading")).render(4) + ")",
             "  val review = draft.ifDefinedNode(d => " + Call("div", c(), Call("keyValue", f'KeyValueProps(List(pair("label", {H}.label(d)), pair("score", {H}.score(d).toString), pair("verdict", verdict)))'),
                                                                Call("small", c(), "if dirty then \"unsaved changes\" else \"clean\"")).render(4) + ")",
             "  val body = step match",
             "    case EditStep.Fields => editor",
             "    case EditStep.Review => review",
             "    case EditStep.Done => " + Call("emptyState", f'EmptyStateProps("saved", Some(primaryButton("again", setStep(EditStep.Fields))))').render(6),
             "  " + Call("pageShell", f'PageShellProps({ctx.show_text("edit ${props.id.raw}")}, props.actions, List("home", "{area.lower()}", "edit"))').render(2) + "(",
             "    " + Call("div", c(),
                          Call("enumTabs", "EditStep.values.toList", "step", "s => setStep(s)"),
                          Call("headerTag", c(), Call("strong", c(), "stepLabel"), Call("small", c(), "verdict")),
                          "body",
                          Call("footer", c(), Call("small", c(), ctx.show_text("${slice.errors.length} errors")))).render(4) + ")",
             "", "enum EditStep:", "  case Fields, Review, Done"]
    ctx.write(f"page.{area.lower()}", f"{area}EditPage", "\n".join(lines) + "\n", extra=imports_for(area))
    ctx.pages.append((area, f"{lower(area)}EditPage", idn))


def write_dashboard(ctx, area, decls, entity):
    c = ctx.classes
    A = f"{area}Actions"
    status, enum_decl = status_of(decls, entity)
    amounts = [n for n, t in entity.fields if t.expr == "Amount"][:1]
    numbers = [n for n, t in entity.fields if t.expr in ("Int", "Long")][:3]
    bools = [n for n, t in entity.fields if t.expr == "Boolean"][:2]
    lines = [f"val {lower(area)}Dashboard: FC[Actions] = FC[Actions]: actions =>",
             f"  val slice = useStore(Environment.store)(_.{lower(area)})",
             "  val items = slice.items.values.toList",
             "  val (period, setPeriod) = useState(Period.Week)",
             "  val (detail, setDetail) = useState(false)",
             "  val periodLabel = period match", "    case Period.Day => \"today\"", "    case Period.Week => \"this week\"", "    case Period.Month => \"this month\""]
    if status and enum_decl:
        lines.append(f"  val byStatus = items.groupBy(_.{status[0]}).map((k, v) => (k, v.length))")
        lines += enum_match(enum_decl, "k", [f"Tone.{t}" for t in TONES], "toneOf", indent="  ")
        lines[-len(enum_decl.cases[:8]) - (2 if len(enum_decl.cases) > 8 else 1)] = f"  def tone(k: {enum_decl.name}): Tone = k match"
    stats = ['StatProps("items", items.length.toLong)', 'StatProps("selected", slice.selected.size.toLong, Some(1L))', 'StatProps("errors", slice.errors.length.toLong)']
    for n in numbers:
        stats.append(f'StatProps("{n}", items.map(_.{n}.toLong).sum, unit = Some("total"))')
    for b in bools:
        stats.append(f'StatProps("{b}", items.count(_.{b}).toLong)')
    if amounts:
        lines.append(f"  val spent = total(items.map(_.{amounts[0]}))")
    parts = [Call("enumTabs", "Period.values.toList", "period", "p => setPeriod(p)"), Call("statsRow", "List(" + ", ".join(stats[:4]) + ")")]
    if len(stats) > 4:
        parts.append(Call("statsRow", "List(" + ", ".join(stats[4:]) + ")"))
    if amounts:
        parts.append(Call("amountLabel", "AmountLabelProps(spent, emphasis = true)"))
    if status and enum_decl:
        label = ctx.show_text("${k.entryName} $n") if enum_decl.has("Enumerated") else ctx.show_text("$k $n")
        parts.append(Call("div", c(), f"byStatus.toList.sortBy(_._2).toKeyedNodes(_._1.toString)((k, n) => badge(BadgeProps({label}, tone(k))))"))
    parts.append(Call("primaryButton", '"refresh"', f"actions.run({A}.refresh)"))
    parts.append(Call("Tags.button", c(), "onClick --> setDetail(!detail)", 'if detail then "summary" else "detail"'))
    parts.append(Call("small", c(), "periodLabel"))
    parts.append("fragmentGate(detail)(" + Call("ul", c(), "items.take(5).toKeyedNodes(_.id.raw.toString)(item => " + Call("li", c(), "item.id.raw.toString").render() + ")").render(4) + ")")
    lines.append("  " + Call("pageShell", f'PageShellProps("{area} overview", actions, List("home", "{area.lower()}", "overview"))').render(2) + "(")
    lines.append("    " + Call("div", c(), *parts).render(4) + ")")
    lines += ["", "enum Period:", "  case Day, Week, Month"]
    ctx.write(f"page.{area.lower()}", f"{area}Dashboard", "\n".join(lines) + "\n", extra=imports_for(area))
    ctx.pages.append((area, f"{lower(area)}Dashboard", None))
