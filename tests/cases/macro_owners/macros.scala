package demo.owners

import scala.quoted.*

final case class Site(chain: String)
object Site:
  inline given generate: Site = ${ Owners.siteImpl }

object Owners:
  inline def chain: String = ${ chainImpl }

  def describe(using Quotes)(s: quotes.reflect.Symbol): String =
    import quotes.reflect.*
    val kinds = List(
      "def" -> s.isDefDef, "val" -> s.isValDef, "class" -> s.isClassDef, "pkg" -> s.isPackageDef,
      "dummy" -> s.isLocalDummy, "ctor" -> s.isClassConstructor, "type" -> s.isType, "term" -> s.isTerm
    ).collect { case (k, true) => k }
    val flags = List(
      "Synthetic" -> Flags.Synthetic, "Method" -> Flags.Method, "Module" -> Flags.Module,
      "Lazy" -> Flags.Lazy, "Given" -> Flags.Given, "Macro" -> Flags.Macro, "Package" -> Flags.Package,
      "Mutable" -> Flags.Mutable, "Case" -> Flags.Case, "Inline" -> Flags.Inline
    ).collect { case (k, f) if s.flags.is(f) => k }
    // Synthetic positions and the numbers of pattern temporaries are left out.
    val synthetic = s.flags.is(Flags.Synthetic)
    val temp = synthetic && s.name.startsWith("$") && s.name != "$anonfun"
    val pos =
      if synthetic || s.isPackageDef then ""
      else s.pos.map(p => s" @${p.startLine + 1}:${p.startColumn}-${p.endLine + 1}:${p.endColumn}").getOrElse(" @-")
    val name = if temp then "<temp>" else s.name
    val fullName = if temp then s.fullName.replaceAll("\\$\\d+$", "\\$N") else s.fullName
    s"$name [${kinds.mkString(",")}] {${flags.mkString(",")}} $fullName$pos"

  def render(using Quotes): String =
    import quotes.reflect.*
    var s = Symbol.spliceOwner
    val out = List.newBuilder[String]
    while s != Symbol.noSymbol && s != defn.RootClass && s != defn.RootPackage do
      out += describe(s)
      s = s.maybeOwner
    out.result().mkString("\n  ", "\n  ", "")

  def chainImpl(using Quotes): Expr[String] = Expr(render)
  def siteImpl(using Quotes): Expr[Site] = '{ Site(${ Expr(render) }) }
