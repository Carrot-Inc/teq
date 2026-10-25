// closure.py's construction sites (../README.md, "The graph"): a scalac 3.8.4 plugin phase run after the typer in
// semanticdb.sh's compile of dotty's sources. SemanticDB names the class at a construction, never the constructor the
// typer selected (ExtractSemanticDB.scala's Select case registers no use of a `<init>` selection, which has no name
// span), so the typed tree gives it: each constructor call, one a line,
//   <kind> <source path> <line> <the constructor's SemanticDB symbol>
// `new` for `new C(..)`, a creator application `C(..)` or `C[T](..)` (a constructor proxy, `new` after the typer) and
// an anonymous class's parent call (the instance's construction runs it); `self` for a secondary constructor's
// `this(..)`; `parent` for a named class's parent clause, its own construction's; `call` for an `apply` the typer inserted
// on a value (`f(x)` as `f.apply(x)`), which SemanticDB does not record either. The symbol is SemanticDB's own
// (`SemanticSymbolBuilder`, the reason this file is in its package), so that it names the graph's node; a local class's
// constructor is left out (no list holds it).
package dotty.tools.dotc.semanticdb

import dotty.tools.dotc.ast.tpd.*
import dotty.tools.dotc.core.Contexts.*
import dotty.tools.dotc.core.Symbols.*
import dotty.tools.dotc.plugins.{PluginPhase, StandardPlugin}
import dotty.tools.dotc.typer.TyperPhase

import java.io.{FileWriter, PrintWriter}
import scala.collection.mutable

class Constructions extends StandardPlugin:
  val name = "constructions"
  val description = "closure.py's construction sites: the constructor each call selects"
  override def init(options: List[String]): List[PluginPhase] =
    List(ConstructionsPhase(options.headOption.getOrElse(sys.error("constructions: -P:constructions:<out file>"))))

class ConstructionsPhase(out: String) extends PluginPhase:
  val phaseName = "constructions"
  override val runsAfter = Set(TyperPhase.name)
  override val runsBefore = Set(ExtractSemanticDB.phaseNamePrefix + "ExtractSemanticInfo")

  private val names = SemanticSymbolBuilder()
  private lazy val sink = PrintWriter(FileWriter(out, true))

  /** The constructor's selection a call is built on, through its argument lists, type arguments and the block of its
   *  lifted arguments. */
  private def selection(t: Tree): Tree = t match
    case Apply(f, _) => selection(f)
    case TypeApply(f, _) => selection(f)
    case Block(_, e) => selection(e)
    case _ => t

  // The whole unit by a traverser of its own, which reaches every tree the typer left (a named argument's among them,
  // which the mini-phases' traversal does not visit), each construction recorded once, at the selection of its
  // constructor (`new C` or `this` for `<init>`), whatever argument lists the call has.
  override def prepareForUnit(tree: Tree)(using Context): Context =
    val source = ctx.compilationUnit.source
    val parents = mutable.HashMap[Tree, String]()  // a template's parent calls' selections, by identity: their kind
    def emit(kind: String, at: Tree, ctor: Symbol): Unit =
      if ctor.exists && ctor.isConstructor && ctor.owner.isClass && !ctor.owner.isLocal && at.span.exists then
        sink.println(s"$kind ${source.file.path} ${source.offsetToLine(at.span.start) + 1} ${names.symbolName(ctor)}")
    // An `apply` the typer inserted on a value (`f(x)` for `f.apply(x)`), which SemanticDB records no use of.
    def inserted(sel: Select): Boolean =
      sel.name == dotty.tools.dotc.core.StdNames.nme.apply && sel.span.exists && sel.qualifier.span.exists
        && !source.content.slice(sel.span.point, sel.span.point + 5).sameElements("apply".toCharArray)
    new TreeTraverser:
      def traverse(t: Tree)(using Context): Unit =
        t match
          case cdef @ TypeDef(_, tmpl: Template) =>
            val anon = cdef.symbol.isAnonymousClass
            for p <- tmpl.parents do
              p match
                // An anonymous class's trait parent: the instance's construction runs the trait's initializer.
                case _: TypeTree | _: Ident | _: Select if anon && p.tpe.classSymbol.is(dotty.tools.dotc.core.Flags.Trait) =>
                  emit("new", p, p.tpe.classSymbol.primaryConstructor)
                case _ => parents(selection(p)) = if anon then "new" else "parent"
          case sel: Select if inserted(sel) && sel.symbol.exists && sel.symbol.owner.isClass && !sel.symbol.owner.isLocal =>
            sink.println(s"call ${source.file.path} ${source.offsetToLine(sel.span.start) + 1} ${names.symbolName(sel.symbol)}")
          case sel @ Select(qual, _) if sel.symbol.isConstructor =>
            emit(parents.remove(sel).getOrElse(qual match
              case New(_) => "new"
              case _ => "self"), sel, sel.symbol)
          case id: Ident if id.symbol.isConstructor =>
            emit(parents.remove(id).getOrElse("self"), id, id.symbol)
          case _ =>
        traverseChildren(t)
    .traverse(tree)
    sink.flush()
    ctx
