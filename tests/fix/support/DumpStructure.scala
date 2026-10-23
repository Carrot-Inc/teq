import teq.fix.*

// The document's tree as scalameta's `MetaStructure.scala` prints a tree: a node a line, its field, its class, its
// scalar fields and its syntax, each derived accessor's answer under it, then each child; every field of scalameta
// 4.17.4's class and every derived accessor of std/fix/tree.scala through teq's reader (the node contract,
// docs/FIX.md); one lint, at the definition `val marker`, else at the first import.
final class DumpStructure extends SyntacticRule("DumpStructure"):
  private enum Entry:
    case Child(name: String, value: Any)
    case Scalar(name: String, value: Any)
    case Derived(name: String, value: Any)
    case Failed(name: String, message: String)
  import Entry.*

  private def f(name: String, v: => Any): Entry =
    try Child(name, v)
    catch case e: Exception => Failed(name, e.getClass.getSimpleName + ": " + e.getMessage)
  private def s(name: String, v: => Any): Entry =
    try Scalar(name, v)
    catch case e: Exception => Failed(name, e.getClass.getSimpleName + ": " + e.getMessage)
  private def d(name: String, v: => Any): Entry =
    try Derived(name, v)
    catch case e: Exception => Failed("." + name, e.getClass.getSimpleName + ": " + e.getMessage)
  private def node(entries: List[Entry]): List[Entry] = entries

  private def quoted(s: String): String =
    val b = new StringBuilder("\"")
    s.foreach {
      case '"' => b ++= "\\\""
      case '\\' => b ++= "\\\\"
      case '\n' => b ++= "\\n"
      case '\r' => b ++= "\\r"
      case '\t' => b ++= "\\t"
      case c if c < ' ' => b ++= "\\u%04x".format(c.toInt)
      case c => b += c
    }
    b += '"'
    b.toString

  private def scalar(v: Any): String = v match
    case s: String => quoted(s)
    case c: Char => quoted(c.toString)
    case x => String.valueOf(x)

  /** A derived accessor's answer: a node its class and syntax, a list, an option and a pair of them, a scalar. */
  private def render(v: Any): String = v match
    case t: Tree => t.kind + " " + quoted(t.syntax)
    case l: List[?] => l.map(render).mkString("[", ", ", "]")
    case None => "None"
    case Some(x) => "Some(" + render(x) + ")"
    case (a, b) => "(" + render(a) + ", " + render(b) + ")"
    case () => "()"
    case null => "null"
    case x => scalar(x)

  override def fix(using doc: SyntacticDocument): Patch =
    val out = new StringBuilder
    def walk(field: String, t: Tree, depth: Int): Unit =
      val entries = fields(t)
      val scalars = entries.collect { case Scalar(n, v) => " " + n + "=" + scalar(v) }.mkString
      out ++= "  " * depth + field + " " + t.kind + scalars + " syntax=" + quoted(t.syntax) + "\n"
      val pad = "  " * (depth + 1)
      derived(t).foreach {
        case Derived(n, v) => out ++= pad + "." + n + " = " + render(v) + "\n"
        case Failed(n, m) => out ++= pad + n + " !" + m + "\n"
        case _ =>
      }
      entries.foreach {
        case Child(n, c: Tree) => walk(n, c, depth + 1)
        case Child(n, l: List[?]) if l.isEmpty => out ++= pad + n + " Nil\n"
        case Child(n, l: List[?]) => l.foreach { case c: Tree => walk(n, c, depth + 1) }
        case Child(n, None) => out ++= pad + n + " None\n"
        case Child(n, Some(c: Tree)) => walk(n, c, depth + 1)
        case Failed(n, m) => out ++= pad + n + " !" + m + "\n"
        case _ =>
      }
    walk("-", doc.tree, 0)
    val marker = doc.tree.collect { case t: Defn.Val if t.syntax.startsWith("val marker") => t.pos }.headOption
    val at = marker.orElse(doc.tree.collect { case t: Import => t.pos }.headOption).getOrElse(doc.tree.pos)
    Patch.lint(Diagnostic("structure", "BEGIN\n" + out.toString + "END", at))

  // Every class of scalameta 4.17.4's Trees.scala teq makes, its fields in that file's order.
  private def fields(t: Tree): List[Entry] = t match
      case t: Stat.Block => node(List(f("stats", t.stats)))
      case t: Name.Anonymous => node(List())
      case t: Name.This => node(List())
      case t: Name.Indeterminate => node(List(s("value", t.value)))
      case t: Name.Placeholder => node(List())
      case t: Lit.Null => node(List())
      case t: Lit.Int => node(List(s("value", t.value)))
      case t: Lit.Double => node(List(s("format", t.format)))
      case t: Lit.Float => node(List(s("format", t.format)))
      case t: Lit.WithUnary => node(List(f("op", t.op), f("arg", t.arg)))
      case t: Lit.Char => node(List(s("value", t.value)))
      case t: Lit.Long => node(List(s("value", t.value)))
      case t: Lit.Boolean => node(List(s("value", t.value)))
      case t: Lit.Unit => node(List())
      case t: Lit.String => node(List(s("value", t.value)))
      case t: Term.ArgClause => node(List(f("values", t.values), f("mod", t.mod)))
      case t: Term.CasesBlock => node(List(f("cases", t.cases)))
      case t: Term.EnumeratorsBlock => node(List(f("enums", t.enums)))
      case t: Term.This => node(List(f("qual", t.qual)))
      case t: Term.Super => node(List(f("thisp", t.thisp), f("superp", t.superp)))
      case t: Term.Name => node(List(s("value", t.value)))
      case t: Term.Anonymous => node(List())
      case t: Term.Select => node(List(f("qual", t.qual), f("name", t.name)))
      case t: Term.SelectPostfix => node(List(f("qual", t.qual), f("name", t.name)))
      case t: Term.Interpolate => node(List(f("prefix", t.prefix), f("parts", t.parts), f("args", t.args)))
      case t: Term.Apply => node(List(f("fun", t.fun), f("argClause", t.argClause)))
      case t: Term.ApplyType => node(List(f("fun", t.fun), f("targClause", t.targClause)))
      case t: Term.ApplyInfix => node(List(f("lhs", t.lhs), f("op", t.op), f("targClause", t.targClause), f("argClause", t.argClause)))
      case t: Term.ApplyUnary => node(List(f("op", t.op), f("arg", t.arg)))
      case t: Term.Assign => node(List(f("lhs", t.lhs), f("rhs", t.rhs)))
      case t: Term.Return => node(List(f("expr", t.expr)))
      case t: Term.Throw => node(List(f("expr", t.expr)))
      case t: Term.Ascribe => node(List(f("expr", t.expr), f("tpe", t.tpe)))
      case t: Term.Annotate => node(List(f("expr", t.expr), f("annots", t.annots)))
      case t: Term.Tuple => node(List(f("args", t.args)))
      case t: Term.Block => node(List(f("stats", t.stats)))
      case t: Term.EndMarker => node(List(f("name", t.name)))
      case t: Term.If => node(List(f("cond", t.cond), f("thenp", t.thenp), f("elsep", t.elsep), f("mods", t.mods)))
      case t: Term.QuotedMacroExpr => node(List(f("body", t.body)))
      case t: Term.QuotedMacroType => node(List(f("tpe", t.tpe)))
      case t: Term.SplicedMacroExpr => node(List(f("body", t.body)))
      case t: Term.SplicedMacroPat => node(List(f("pat", t.pat)))
      case t: Term.Match => node(List(f("expr", t.expr), f("casesBlock", t.casesBlock), f("mods", t.mods)))
      case t: Term.SelectMatch => node(List(f("expr", t.expr), f("casesBlock", t.casesBlock), f("mods", t.mods)))
      case t: Term.Try => node(List(f("expr", t.expr), f("catchClause", t.catchClause), f("finallyp", t.finallyp)))
      case t: Term.TryWithHandler => node(List(f("expr", t.expr), f("catchp", t.catchp), f("finallyp", t.finallyp)))
      case t: Term.ContextFunction => node(List(f("paramClause", t.paramClause), f("body", t.body)))
      case t: Term.Function => node(List(f("paramClause", t.paramClause), f("body", t.body)))
      case t: Term.AnonymousFunction => node(List(f("body", t.body)))
      case t: Term.PolyFunction => node(List(f("tparamClause", t.tparamClause), f("body", t.body)))
      case t: Term.PartialFunction => node(List(f("cases", t.cases)))
      case t: Term.While => node(List(f("expr", t.expr), f("body", t.body)))
      case t: Term.For => node(List(f("enumsBlock", t.enumsBlock), f("body", t.body)))
      case t: Term.ForYield => node(List(f("enumsBlock", t.enumsBlock), f("body", t.body)))
      case t: Term.New => node(List(f("init", t.init)))
      case t: Term.NewAnonymous => node(List(f("templ", t.templ)))
      case t: Term.Placeholder => node(List())
      case t: Term.Eta => node(List(f("expr", t.expr)))
      case t: Term.Repeated => node(List(f("expr", t.expr)))
      case t: Term.Param => node(List(f("mods", t.mods), f("name", t.name), f("decltpe", t.decltpe), f("default", t.default)))
      case t: Term.ParamClause => node(List(f("values", t.values), f("mod", t.mod)))
      case t: Type.ArgClause => node(List(f("values", t.values)))
      case t: Type.CasesBlock => node(List(f("cases", t.cases)))
      case t: Type.Name => node(List(s("value", t.value)))
      case t: Type.Select => node(List(f("qual", t.qual), f("name", t.name)))
      case t: Type.Project => node(List(f("qual", t.qual), f("name", t.name)))
      case t: Type.Singleton => node(List(f("ref", t.ref)))
      case t: Type.Apply => node(List(f("tpe", t.tpe), f("argClause", t.argClause)))
      case t: Type.ApplyInfix => node(List(f("lhs", t.lhs), f("op", t.op), f("rhs", t.rhs)))
      case t: Type.FuncParamClause => node(List(f("values", t.values)))
      case t: Type.Function => node(List(f("paramClause", t.paramClause), f("res", t.res)))
      case t: Type.PolyFunction => node(List(f("tparamClause", t.tparamClause), f("tpe", t.tpe)))
      case t: Type.ContextFunction => node(List(f("paramClause", t.paramClause), f("res", t.res)))
      case t: Type.Tuple => node(List(f("args", t.args)))
      case t: Type.With => node(List(f("lhs", t.lhs), f("rhs", t.rhs)))
      case t: Type.AnonymousLambda => node(List(f("tpe", t.tpe)))
      case t: Type.AnonymousParam => node(List(f("variant", t.variant)))
      case t: Type.Refine => node(List(f("tpe", t.tpe), f("body", t.body)))
      case t: Type.Annotate => node(List(f("tpe", t.tpe), f("annots", t.annots)))
      case t: Type.Lambda => node(List(f("tparamClause", t.tparamClause), f("tpe", t.tpe)))
      case t: Type.PatWildcard => node(List())
      case t: Type.Wildcard => node(List(f("bounds", t.bounds)))
      case t: Type.Bounds => node(List(f("lo", t.lo), f("hi", t.hi), f("context", t.context), f("view", t.view)))
      case t: Type.BoundsAlias => node(List(f("name", t.name), f("bounds", t.bounds)))
      case t: Type.ByName => node(List(f("tpe", t.tpe)))
      case t: Type.Repeated => node(List(f("tpe", t.tpe)))
      case t: Type.Var => node(List(f("name", t.name)))
      case t: Type.TypedParam => node(List(f("name", t.name), f("typ", t.typ), f("mods", t.mods)))
      case t: Type.Param => node(List(f("mods", t.mods), f("name", t.name), f("tparamClause", t.tparamClause), f("bounds", t.bounds)))
      case t: Type.ParamClause => node(List(f("values", t.values)))
      case t: Type.Match => node(List(f("tpe", t.tpe), f("casesBlock", t.casesBlock)))
      case t: Pat.ArgClause => node(List(f("values", t.values)))
      case t: Pat.Var => node(List(f("name", t.name)))
      case t: Pat.Wildcard => node(List())
      case t: Pat.SeqWildcard => node(List())
      case t: Pat.Bind => node(List(f("lhs", t.lhs), f("rhs", t.rhs)))
      case t: Pat.Alternative => node(List(f("lhs", t.lhs), f("rhs", t.rhs)))
      case t: Pat.Tuple => node(List(f("args", t.args)))
      case t: Pat.Repeated => node(List(f("name", t.name)))
      case t: Pat.Extract => node(List(f("fun", t.fun), f("argClause", t.argClause)))
      case t: Pat.ExtractInfix => node(List(f("lhs", t.lhs), f("op", t.op), f("argClause", t.argClause)))
      case t: Pat.Interpolate => node(List(f("prefix", t.prefix), f("parts", t.parts), f("args", t.args)))
      case t: Pat.Typed => node(List(f("lhs", t.lhs), f("rhs", t.rhs)))
      case t: Pat.Assign => node(List(f("name", t.name), f("rhs", t.rhs)))
      case t: Pat.Macro => node(List(f("body", t.body)))
      case t: Pat.Given => node(List(f("tpe", t.tpe)))
      case t: Member.ParamClauseGroup => node(List(f("tparamClause", t.tparamClause), f("paramClauses", t.paramClauses)))
      case t: Decl.Val => node(List(f("mods", t.mods), f("pats", t.pats), f("decltpe", t.decltpe)))
      case t: Decl.Var => node(List(f("mods", t.mods), f("pats", t.pats), f("decltpe", t.decltpe)))
      case t: Decl.Def => node(List(f("mods", t.mods), f("name", t.name), f("paramClauseGroups", t.paramClauseGroups), f("decltpe", t.decltpe)))
      case t: Decl.Type => node(List(f("mods", t.mods), f("name", t.name), f("tparamClause", t.tparamClause), f("bounds", t.bounds)))
      case t: Decl.Given => node(List(f("mods", t.mods), f("name", t.name), f("paramClauseGroups", t.paramClauseGroups), f("decltpe", t.decltpe)))
      case t: Decl.GivenAnonymous => node(List(f("mods", t.mods), f("name", t.name), f("paramClauseGroups", t.paramClauseGroups), f("decltpe", t.decltpe)))
      case t: Defn.Val => node(List(f("mods", t.mods), f("pats", t.pats), f("decltpe", t.decltpe), f("rhs", t.rhs)))
      case t: Defn.Var => node(List(f("mods", t.mods), f("pats", t.pats), f("decltpe", t.decltpe), f("body", t.body)))
      case t: Defn.Given => node(List(f("mods", t.mods), f("name", t.name), f("paramClauseGroups", t.paramClauseGroups), f("templ", t.templ)))
      case t: Defn.Enum => node(List(f("mods", t.mods), f("name", t.name), f("tparamClause", t.tparamClause), f("ctor", t.ctor), f("templ", t.templ)))
      case t: Defn.EnumCase => node(List(f("mods", t.mods), f("name", t.name), f("tparamClause", t.tparamClause), f("ctor", t.ctor), f("inits", t.inits)))
      case t: Defn.RepeatedEnumCase => node(List(f("mods", t.mods), f("cases", t.cases)))
      case t: Defn.GivenAlias => node(List(f("mods", t.mods), f("name", t.name), f("paramClauseGroups", t.paramClauseGroups), f("decltpe", t.decltpe), f("body", t.body)))
      case t: Defn.ExtensionGroup => node(List(f("paramClauseGroup", t.paramClauseGroup), f("body", t.body)))
      case t: Defn.Def => node(List(f("mods", t.mods), f("name", t.name), f("paramClauseGroups", t.paramClauseGroups), f("decltpe", t.decltpe), f("body", t.body)))
      case t: Defn.Type => node(List(f("mods", t.mods), f("name", t.name), f("tparamClause", t.tparamClause), f("body", t.body), f("bounds", t.bounds)))
      case t: Defn.Class => node(List(f("mods", t.mods), f("name", t.name), f("tparamClause", t.tparamClause), f("ctor", t.ctor), f("templ", t.templ)))
      case t: Defn.Trait => node(List(f("mods", t.mods), f("name", t.name), f("tparamClause", t.tparamClause), f("ctor", t.ctor), f("templ", t.templ)))
      case t: Defn.Object => node(List(f("mods", t.mods), f("name", t.name), f("templ", t.templ)))
      case t: Pkg => node(List(f("ref", t.ref), f("body", t.body)))
      case t: Pkg.Object => node(List(f("mods", t.mods), f("name", t.name), f("templ", t.templ)))
      case t: Pkg.Body => node(List(f("stats", t.stats)))
      case t: Ctor.Primary => node(List(f("mods", t.mods), f("name", t.name), f("paramClauses", t.paramClauses)))
      case t: Ctor.Block => node(List(f("init", t.init), f("stats", t.stats)))
      case t: Ctor.Secondary => node(List(f("mods", t.mods), f("name", t.name), f("paramClauses", t.paramClauses), f("body", t.body)))
      case t: Init => node(List(f("tpe", t.tpe), f("name", t.name), f("argClauses", t.argClauses)))
      case t: Self => node(List(f("name", t.name), f("decltpe", t.decltpe)))
      case t: Template.Body => node(List(f("selfOpt", t.selfOpt), f("stats", t.stats)))
      case t: Template => node(List(f("earlyClause", t.earlyClause), f("inits", t.inits), f("body", t.body), f("derives", t.derives)))
      case t: Mod.Annot => node(List(f("init", t.init)))
      case t: Mod.Private => node(List(f("within", t.within)))
      case t: Mod.Protected => node(List(f("within", t.within)))
      case t: Mod.Implicit => node(List())
      case t: Mod.Final => node(List())
      case t: Mod.Sealed => node(List())
      case t: Mod.Open => node(List())
      case t: Mod.Override => node(List())
      case t: Mod.Case => node(List())
      case t: Mod.Abstract => node(List())
      case t: Mod.Covariant => node(List())
      case t: Mod.Contravariant => node(List())
      case t: Mod.Lazy => node(List())
      case t: Mod.ValParam => node(List())
      case t: Mod.VarParam => node(List())
      case t: Mod.Infix => node(List())
      case t: Mod.Inline => node(List())
      case t: Mod.Using => node(List())
      case t: Mod.Opaque => node(List())
      case t: Mod.Transparent => node(List())
      case t: Enumerator.Generator => node(List(f("pat", t.pat), f("rhs", t.rhs)))
      case t: Enumerator.CaseGenerator => node(List(f("pat", t.pat), f("rhs", t.rhs)))
      case t: Enumerator.Val => node(List(f("pat", t.pat), f("rhs", t.rhs)))
      case t: Enumerator.Guard => node(List(f("cond", t.cond)))
      case t: Import => node(List(f("importers", t.importers)))
      case t: Export => node(List(f("importers", t.importers)))
      case t: Importer => node(List(f("ref", t.ref), f("importees", t.importees)))
      case t: Importee.Wildcard => node(List())
      case t: Importee.Given => node(List(f("tpe", t.tpe)))
      case t: Importee.GivenAll => node(List())
      case t: Importee.Name => node(List(f("name", t.name)))
      case t: Importee.Rename => node(List(f("name", t.name), f("rename", t.rename)))
      case t: Importee.Unimport => node(List(f("name", t.name)))
      case t: Case => node(List(f("pat", t.pat), f("cond", t.cond), f("body", t.body)))
      case t: TypeCase => node(List(f("pat", t.pat), f("body", t.body)))
      case t: Source => node(List(f("stats", t.stats)))
      case _ => List(Failed("class", "no case of " + t.kind))

  // Every derived accessor of std/fix/tree.scala, by class, MetaStructure.scala's list.
  private def derived(t: Tree): List[Entry] = t match
      case t: Decl.Def => List(d("paramClauses", t.paramClauses), d("paramss", t.paramss), d("tparams", t.tparams))
      case t: Decl.Given => List(d("paramClauses", t.paramClauses), d("tparams", t.tparams))
      case t: Decl.Type => List(d("tparams", t.tparams))
      case t: Defn.Class => List(d("tparams", t.tparams))
      case t: Defn.Def => List(d("paramClauses", t.paramClauses), d("paramss", t.paramss), d("tparams", t.tparams))
      case t: Defn.Enum => List(d("tparams", t.tparams))
      case t: Defn.EnumCase => List(d("tparams", t.tparams))
      case t: Defn.Given => List(d("paramClauses", t.paramClauses), d("tparams", t.tparams))
      case t: Defn.GivenAlias => List(d("paramClauses", t.paramClauses), d("tparams", t.tparams))
      case t: Defn.Trait => List(d("tparams", t.tparams))
      case t: Defn.Type => List(d("tparams", t.tparams))
      case t: Defn.Var => List(d("rhs", t.rhs))
      case t: Init => List(d("argss", t.argss))
      case t: Lit.Double => List(d("value", t.value))
      case t: Lit.Float => List(d("value", t.value))
      case t: Lit.Null => List(d("value", t.value))
      case t: Lit.Unit => List(d("value", t.value))
      case t: Lit.WithUnary => List(d("value", t.value))
      case t: Name.Anonymous => List(d("value", t.value))
      case t: Name.Placeholder => List(d("value", t.value))
      case t: Name.This => List(d("value", t.value))
      case t: Pat.Extract => List(d("args", t.args))
      case t: Pat.ExtractInfix => List(d("rhs", t.rhs))
      case t: Pkg => List(d("stats", t.stats))
      case t: Template => List(d("self", t.self), d("stats", t.stats))
      case t: Term.Apply => List(d("args", t.args))
      case t: Term.ApplyInfix => List(d("args", t.args), d("targs", t.targs))
      case t: Term.ApplyType => List(d("targs", t.targs))
      case t: Term.ContextFunction => List(d("params", t.params))
      case t: Term.For => List(d("enums", t.enums))
      case t: Term.ForYield => List(d("enums", t.enums))
      case t: Term.Function => List(d("params", t.params))
      case t: Term.Match => List(d("cases", t.cases))
      case t: Term.PolyFunction => List(d("tparams", t.tparams))
      case t: Term.SelectMatch => List(d("cases", t.cases))
      case t: Term.Try => List(d("catchp", t.catchp))
      case t: Type.Apply => List(d("args", t.args))
      case t: Type.Lambda => List(d("tparams", t.tparams))
      case t: Type.Match => List(d("cases", t.cases))
      case t: Type.Param => List(d("tparams", t.tparams))
      case t: Type.PolyFunction => List(d("tparams", t.tparams))
      case t: Type.Refine => List(d("stats", t.stats))
      case _ => Nil
