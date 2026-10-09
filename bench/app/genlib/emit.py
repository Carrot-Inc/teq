"""Scala text for the declarations of the universe."""
from .universe import PRIMS


def derives_clause(decl):
    return f" derives {', '.join(decl.derives)}" if decl.derives else ""


def render_id(decl):
    raw = "Long" if decl.raw_type == "Long" else "String"
    key = "LongKey" if raw == "Long" else "TextKey"
    return f"final case class {decl.name}(value: {raw}) derives {key}\n"


def render_wrapper(decl, u):
    inner = decl.raw_type
    codec = {"String": "JsonCodec.string", "Long": "JsonCodec.long", "Int": "JsonCodec.int"}[inner]
    lines = [f"final case class {decl.name}(value: {inner})" + (":" if decl.members else "")]
    lines += [f"  {m}" for m in decl.members]
    lines.append(f"object {decl.name}:")
    for line in decl.companion:
        lines.append(f"  {line}")
    if "codec" in decl.givens:
        lines.append(f"  given JsonCodec[{decl.name}] = {codec}.transform({decl.name}(_), _.value)")
    if "validated" in decl.givens:
        lines.append(f"  def from(value: {inner}): Either[String, {decl.name}] = if {decl.check} then Right({decl.name}(value)) else Left(\"invalid {decl.name}\")")
        lines.append(f"  given JsonCodec[{decl.name}] = {codec}.transformOrFail(from, _.value)")
    if "schema" in decl.givens:
        schema = {"String": "Schema.string", "Long": "Schema.schemaForLong", "Int": "Schema.schemaForInt"}[inner]
        lines.append(f"  given Schema[{decl.name}] = {schema}.as[{decl.name}]")
    if "eq" in decl.givens:
        lines.append(f"  given Eq[{decl.name}] = Eq.by(_.value)")
    if "order" in decl.givens:
        lines.append(f"  given Order[{decl.name}] = Order.by(_.value)")
    if "show" in decl.givens:
        lines.append(f"  given Show[{decl.name}] = Show.show(_.value.toString)")
    if "text" in decl.givens:
        text = {"String": "TextCodec.string", "Long": "TextCodec.long", "Int": "TextCodec.int"}[inner]
        lines.append(f"  given TextCodec[{decl.name}] = {text}.map({decl.name}(_))(_.value)")
    if "field" in decl.givens:
        field = {"String": "JsonFieldEncoder.string", "Long": "JsonFieldEncoder.long", "Int": "JsonFieldEncoder.int"}[inner]
        fieldd = {"String": "JsonFieldDecoder.string", "Long": "JsonFieldDecoder.long", "Int": "JsonFieldDecoder.int"}[inner]
        lines.append(f"  given JsonFieldEncoder[{decl.name}] = {field}.contramap(_.value)")
        lines.append(f"  given JsonFieldDecoder[{decl.name}] = {fieldd}.map({decl.name}(_))")
    return "\n".join(lines) + "\n"


def render_enum(decl):
    lines = []
    if decl.discriminator:
        lines.append(f'@jsonDiscriminator("{decl.discriminator}")')
    if decl.labelled:
        lines.append(f"enum {decl.name}(val label: String){derives_clause(decl)}:")
        for case in decl.cases:
            lines.append(f'  case {case} extends {decl.name}("{case.lower()}")')
    else:
        lines.append(f"enum {decl.name}{derives_clause(decl)}:")
        chunk = []
        for case in decl.cases:
            chunk.append(case)
            if len(chunk) == 6:
                lines.append("  case " + ", ".join(chunk))
                chunk = []
        if chunk:
            lines.append("  case " + ", ".join(chunk))
    for m in decl.members:
        lines.append(f"  {m}")
    return "\n".join(lines) + "\n"


def render_adt(decl):
    lines = []
    if decl.discriminator:
        lines.append(f'@jsonDiscriminator("{decl.discriminator}")')
    lines.append(f"enum {decl.name}{derives_clause(decl)}:")
    for case in decl.cases:
        fields = decl.case_fields.get(case, [])
        if fields:
            lines.append(f"  case {case}({', '.join(f'{n}: {t.expr}' for n, t in fields)})")
        else:
            lines.append(f"  case {case}")
    for m in decl.members:
        lines.append(f"  {m}")
    return "\n".join(lines) + "\n"


def render_case_class(decl):
    lines = []
    if decl.discriminator:
        lines.append(f'@jsonDiscriminator("{decl.discriminator}")')
    if len(decl.fields) <= 3 and not decl.members:
        params = ", ".join(f"{n}: {t.expr}" for n, t in decl.fields)
        lines.append(f"final case class {decl.name}({params}){derives_clause(decl)}")
    else:
        lines.append(f"final case class {decl.name}(")
        for i, (n, t) in enumerate(decl.fields):
            comma = "," if i < len(decl.fields) - 1 else ""
            lines.append(f"  {n}: {t.expr}{comma}")
        lines.append(f"){derives_clause(decl)}" + (":" if decl.members else ""))
        for m in decl.members:
            lines.append(f"  {m}")
    if decl.companion:
        lines.append(f"object {decl.name}:")
        for line in decl.companion:
            lines.append(f"  {line}")
    return "\n".join(lines) + "\n"


def render_decl(decl, u):
    if decl.kind == "id":
        return render_id(decl)
    if decl.kind == "wrapper":
        return render_wrapper(decl, u)
    if decl.kind == "enum":
        return render_enum(decl)
    if decl.kind == "adt":
        return render_adt(decl)
    return render_case_class(decl)


def sample_value(decl, u, rng):
    """The sample value of a declaration in Scala, its fields' samples drawn from the universe."""
    if decl.kind == "id":
        return u.type_of(decl).sample(rng)
    if decl.kind == "wrapper":
        return u.type_of(decl).sample(rng)
    if decl.kind == "enum":
        return f"{decl.name}.{decl.cases[0]}"
    if decl.kind == "adt":
        case = decl.cases[0]
        fields = decl.case_fields.get(case, [])
        if fields:
            return f"{decl.name}.{case}({', '.join(t.sample(rng, 1) for _, t in fields)})"
        return f"{decl.name}.{case}"
    args = ", ".join(t.sample(rng, 1) for _, t in decl.fields)
    return f"{decl.name}({args})"
