// The public accessors observe.scala accounts for (README.md, "Coverage"), as scalac 3.8.4 reads its own
// `scala.quoted.Quotes`: every public `def` of every `<T>Module` and `<T>Methods` trait of `reflectModule` whose `T` is
// one of the types the observation reaches: below `Tree` or `TypeRepr` (Quotes.scala 282-2421, 2678-3470), below
// `Constant`, `ParamClause` or `Selector` (the leaves and clauses the trees hold), and `Symbol`, `Flags` and `Signature`
// (what it reads off them). One line per `<trait>.<name>`, overloads once, sorted. Run by
// Oracle.scala; oracle.py's `coverage` reads it against coverage.txt.
package dotty.tools.dotc.core

import Contexts.*, Types.*, Symbols.*, Names.*, Flags.*

object Api:
  def main(args: Array[String]): Unit =
    val base = new ContextBase
    val init = base.initialCtx.fresh
    init.setSetting(init.settings.Yusejavacp, true)
    given Context = init
    ctx.initialize()

    val reflect = requiredClass("scala.quoted.Quotes").info.decl(typeName("reflectModule")).symbol.asClass
    val decls = reflect.info.decls.toList
    // Each abstract type member and the type members its upper bound names, `A & B` taken apart.
    def named(t: Type): List[String] = t match
      case AndType(a, b) => named(a) ++ named(b)
      case tr: TypeRef if tr.symbol.owner == reflect => List(tr.symbol.name.toString)
      case _ => Nil
    val parents = decls.collect { case s if s.isType && !s.isClass => s.name.toString -> named(s.info.hiBound) }.toMap
    def below(root: String): Set[String] =
      var found = Set(root)
      var grew = true
      while grew do
        val more = parents.collect { case (t, ps) if !found(t) && ps.exists(found) => t }
        grew = more.nonEmpty
        found ++= more
      found
    // The types the observation reaches: its nodes, their leaves and clauses, and the symbols, flags and signatures it
    // reads off them. No Methods or Module trait of these is set aside whole.
    val kinds = List("Tree", "TypeRepr", "Constant", "ParamClause", "Selector", "Symbol", "Flags", "Signature", "Position")
      .flatMap(below).toSet
    val traits = decls.filter(s => s.isClass && s.is(Trait)).map(_.asClass)
    val lines = for
      t <- traits
      n = t.name.toString
      kind = n.stripSuffix("Module").stripSuffix("Methods")
      if kind != n && kinds(kind)
      d <- t.info.decls.toList
      if d.is(Method) && !d.isConstructor && !d.isOneOf(Private | Protected | Synthetic)
    yield s"$n.${d.name.decode}"
    lines.distinct.sorted.foreach(println)
    // The kinds the serializers' arms must cover: every type below `Tree`, `TypeRepr`, `Constant`, `ParamClause` or
    // `Selector` whose Module has an extractor (`unapply`), by its root.
    val nodes = for
      root <- List("Tree", "TypeRepr", "Constant", "ParamClause", "Selector")
      kind <- below(root).toList.sorted
      module <- traits.find(_.name.toString == kind + "Module")
      if module.info.decls.toList.exists(d => d.name.toString == "unapply" && d.is(Method))
    yield s"kind $root $kind"
    nodes.distinct.foreach(println)
