import teq.fix.*

/** The rule of the passes over the written extents' probes (tests/fix/Driver/ContractProbes): a lint of each
  * `Defn.Given`, of a method `f` whose body is a block, of a term's or a type's cases or a constructor's block holding a
  * comment of its own line and at a missing `else`, a `val f` of more than a line and a method `g` whose body is a
  * block replaced whole, a braced block `{ 1 }` replaced by `2`, a lint of an importee named `given` and a lambda
  * `(x: Int) => x` over more than its `x` replaced whole. */
final class ContractProbes extends SyntacticRule("ContractProbes"):
  override def fix(using doc: SyntacticDocument): Patch =
    doc.tree.collect {
      case t: Defn.Given => Patch.lint(Diagnostic("given", "given definition", t.pos))
      case t: Defn.Def if t.name.value == "f" && t.body.isInstanceOf[Term.Block] => Patch.lint(Diagnostic("block", "block body", t.pos))
      case t: Term.CasesBlock if t.syntax.linesIterator.exists(_.trim.startsWith("//")) => Patch.lint(Diagnostic("cases", "cases holding a comment of its own line", t.pos))
      case t: Type.CasesBlock if t.syntax.linesIterator.exists(_.trim.startsWith("//")) => Patch.lint(Diagnostic("typeCases", "type cases holding a comment of its own line", t.pos))
      case t: Ctor.Block if t.syntax.linesIterator.exists(_.trim.startsWith("//")) => Patch.lint(Diagnostic("ctor", "constructor block holding a comment of its own line", t.pos))
      case t: Term.If if t.elsep.pos.start == t.elsep.pos.end => Patch.lint(Diagnostic("else", "missing else", t.elsep.pos))
      case t: Defn.Def if t.name.value == "g" && t.body.isInstanceOf[Term.Block] => Patch.replaceTree(t, "def g = 0")
      case t: Term.Block if t.syntax == "{ 1 }" => Patch.replaceTree(t, "2")
      case t: Defn.Val if t.syntax.startsWith("val f =") && t.syntax.contains("\n") => Patch.replaceTree(t, "val f = (x: Int) => x")
      case t: Importee.Name if t.syntax == "`given`" => Patch.lint(Diagnostic("importee", "an importee named given", t.pos))
      case t: Term.Function if t.syntax.startsWith("(x: Int) => x") && t.syntax != "(x: Int) => x" => Patch.replaceTree(t, "(x: Int) => 2")
    }.asPatch
