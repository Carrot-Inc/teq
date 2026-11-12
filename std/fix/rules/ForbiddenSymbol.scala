// Adapted from ForbiddenSymbol of scalafix-forbidden-symbol (https://github.com/tanin47/scalafix-forbidden-symbol, commit
// 813a206a4218dc86b52718077fad5d82f2c3d4ed, scalafix/rules/src/main/scala/fix/ScalafixForbiddenSymbol.scala).
//
// Copyright © 2025 Tanin Na Nakorn
//
// Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the “Software”), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED “AS IS”, WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
//
// The port's local changes: the setting `symbols` is decoded by hand where metaconfig derives it; and an entry forbids
// the member it names and every member that overrides it (dotty's `allOverriddenSymbols`), reported where it is
// referenced. Upstream reports a name whose symbol is the entry's alone, so that `xs.last` is reported for the entry
// `scala.collection.IterableOps.last` where `xs` is a `Seq` and not where it is a `List`, whose class overrides it.
// Every lint names the entry that forbids the use, in upstream's sentence; an entry that names no symbol of any
// program of the run is told by the run (docs/FIX.md, "The rules teq ships").
package teq.fix.rules

import teq.fix.*

final class ForbiddenSymbol(symbols: List[String], places: List[String]) extends SemanticRule("ForbiddenSymbol"):
  def this() = this(Nil, Nil)

  override def withConfiguration(c: Configuration): Configured[Rule] =
    val section = c.conf.section("ForbiddenSymbol")
    // Each entry's place in the configuration, where the run tells one that names no symbol.
    val at = section.get("symbols").map(_.items.map(_.place)).getOrElse(Nil)
    section.strings("symbols", symbols).map(read => new ForbiddenSymbol(read, if at.lengthCompare(read.length) == 0 then at else read.map(_ => section.place)))

  /** The entries as `SymbolMatcher.normalized` reads them: a written name a term's, each by its owners' names. */
  private val forbidden: Set[String] = symbols.map(s => SymbolSyntax.normalize(Symbol(SymbolSyntax.inferTrailingDot(s))).value).toSet
  /** The entries' own names: a member that overrides one has its name. */
  private val names: Set[String] = forbidden.map(e => Symbol(e).displayName)
  private val nameList: List[String] = names.toList

  /** The entry that forbids the name: its symbol's own, else the first its symbol overrides, where the name refers
    * to the member and does not define it. A member of a local class is a local symbol, which carries no name: the
    * tree's own name tells whether it can override an entry. */
  private def entryOf(t: Name)(using doc: SemanticDocument): Option[String] =
    val sym = doc.symbol(t)
    if sym.isNone then None
    else if sym.isGlobal then
      // A symbol that is an entry's, or overrides one, ends in an entry's name: the others are not read further.
      if !nameList.exists(n => sym.value.contains(n)) then None
      else
        val normalized = sym.normalized.value
        if forbidden(normalized) then Some(normalized)
        else if names(sym.displayName) && !doc.defines(t) then overridden(sym)
        else None
    else if names(t.value) && !doc.defines(t) then overridden(sym)
    else None

  /** The first entry among the members a symbol overrides. */
  private def overridden(sym: Symbol)(using doc: SemanticDocument): Option[String] =
    doc.overriddenSymbols(sym).map(_.normalized.value).find(forbidden)

  override def fix(using doc: SemanticDocument): Patch =
    symbols.zip(places).foreach((written, place) => doc.names(written, SymbolSyntax.normalize(Symbol(SymbolSyntax.inferTrailingDot(written))).value, place))
    doc.tree.collect {
      case t: Name =>
        entryOf(t) match
          case Some(entry) =>
            Patch.lint(Diagnostic(id = "", message = entry.stripSuffix(".") + " is forbidden according to the ForbiddenSymbol rule.", position = t.pos))
          case None => Patch.empty
    }.asPatch
