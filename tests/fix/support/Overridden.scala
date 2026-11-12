// dotty's own answer for every term reference of a program: the member it names and the members that one overrides
// (`SymDenotations.allOverriddenSymbols`), which scalac's SemanticDB does not record for a library's members.
// tests/fix/capture.sh compiles this file with scalac 3.8.4 against scala3-compiler_3 3.8.4 and runs it over a case's
// sources for the ForbiddenSymbol half of its `--scalac` oracle:
//   java -cp <compiler>:<classes> teq.fix.capture.Overridden <out> -classpath <class path> <sources>
// The program is parsed and typed, nothing more (the phases are the parser, the typer and the one below, at the place
// of dotty's ExtractSemanticInfo); <out> gets a line for each `Ident` and `Select` whose symbol is a term and whose
// name has a length, and for each import selector naming a term, at the span ExtractSemanticDB gives the name's
// occurrence (`selectSpan`, an identifier's own, a selector's `imported`):
//   <file>\t<line>:<col>-<line>:<col>\t<the symbol's fullName>\t<its allOverriddenSymbols' fullNames, ", " between>
// lines and columns from 1, a column a UTF-16 unit as SemanticDB's. A fullName has `$` where an owner is an
// object's class (`scala.Predef$.println`); the reader drops them.
package teq.fix.capture

import dotty.tools.dotc.{Compiler, Driver}
import dotty.tools.dotc.ast.tpd
import dotty.tools.dotc.core.Contexts.{Context, ctx}
import dotty.tools.dotc.core.Phases.Phase
import dotty.tools.dotc.core.StdNames.nme
import dotty.tools.dotc.core.Symbols.*
import dotty.tools.dotc.parsing.Parser
import dotty.tools.dotc.typer.TyperPhase
import dotty.tools.dotc.util.Spans.Span

object Overridden:
  def main(args: Array[String]): Unit =
    if args.length < 2 then
      System.err.println("usage: Overridden <out> <scalac's arguments and sources>")
      sys.exit(2)
    val out = new java.io.PrintWriter(args(0), "UTF-8")
    val failed =
      try new Dump(out).process(args.drop(1)).hasErrors
      finally out.close()
    sys.exit(if failed then 1 else 0)

final class Dump(out: java.io.PrintWriter) extends Driver:
  override protected def newCompiler(using Context): Compiler = new Compiler:
    override def phases: List[List[Phase]] = List(List(new Parser), List(new TyperPhase), List(new DumpPhase(out)))

final class DumpPhase(out: java.io.PrintWriter) extends Phase:
  def phaseName: String = "teqOverridden"

  protected def run(using Context): Unit =
    val source = ctx.compilationUnit.source
    def at(offset: Int): String = s"${source.offsetToLine(offset) + 1}:${source.column(offset) + 1}"
    def row(span: Span, sym: Symbol): Unit =
      if span.exists && span.start < span.end && sym.exists && sym.isTerm then
        val overridden = sym.allOverriddenSymbols.map(_.fullName.toString).mkString(", ")
        out.println(s"${source.file.name}\t${at(span.start)}-${at(span.end)}\t${sym.fullName}\t$overridden")
    // ExtractSemanticDB.selectSpan: the name's span, or what follows the qualifier where the selection has no more.
    def selectSpan(tree: tpd.Select): Span =
      val end = tree.span.end
      val limit = tree.qualifier.span.end
      if limit < end then tree.nameSpan else Span(limit, end)
    val traverser = new tpd.TreeTraverser:
      def traverse(tree: tpd.Tree)(using Context): Unit = tree match
        case tree: tpd.Ident =>
          if tree.name != nme.WILDCARD then row(tree.span, tree.symbol)
        case tree: tpd.Select =>
          traverse(tree.qualifier)
          if tree.span.exists then row(selectSpan(tree), tree.symbol)
        case tree: tpd.Import =>
          traverse(tree.expr)
          for sel <- tree.selectors do
            val imported = sel.imported.name
            if imported != nme.WILDCARD then
              for alt <- tree.expr.tpe.member(imported).alternatives do row(sel.imported.span, alt.symbol)
        // An inline call is its call as written; the expansion's trees are another place's.
        case tree: tpd.Inlined => traverse(tree.call)
        case _ => traverseChildren(tree)
    traverser.traverse(ctx.compilationUnit.tpdTree)
