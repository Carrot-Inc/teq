// InitDump: what scalac 3.8.4's initialization checker reads, printed after two phases, the oracle of the
// initialization view (tests/init-view.sh; the view is src/init/, printed by `TEQ_INIT_VIEW=<root> teq tasty <dir>`).
//
// It is scalac's own Driver, whose Compiler adds a phase of its own right after the `pickler` group and right after
// the `initChecker` group (Compiler.scala: `List(new init.Checker)` is a group of its own, after the RefChecks group).
// After `pickler` the units' trees are the pickled ones (the view's first part reads them as written); after
// `initChecker` they are the trees the checker interprets (FirstTransform's normal forms, ExpandSAMs's classes,
// the Inlined nodes of Inlining). It lives in the checker's package to call `Util`'s own `isConcreteClass` and
// `isStaticObject`.
//
// Per phase and per unit, in this order:
//   `phase <name>` and `unit <source path>`;
//   `class <symbol> concrete=<bool> staticObject=<bool>`: the classes `Checker.InitTreeTraverser` collects, in its
//     post-order (children first, then the class), with `Util.isConcreteClass` and `isStaticObject`;
//   the tree, a line a node, indented by depth, in `tpd.TreeTraverser`'s order: the node's kind, its position, and
//     what the checker reads of it (below). Type trees are one line, `Tpt`, with the paths of their type that
//     `Semantic.checkTermUsage` evaluates; their insides are not printed. Empty trees and imports are not printed.
// A source is printed the first time a position names it: `source s<n> name=<file name> path=<file path>
// content=<yes|no> sha256=<hex of the content or ->`, which is what `Trace.buildStacktrace` reads of it (the name
// where the content exists, the path where it does not: `fileExists` is `content().nonEmpty`).
// A position is `[s<n> <start>..<end>@<point> <line>:<column>-<line>:<column>]` (lines from 1, columns from 0, as
// `SourcePosition` gives them and the trace prints `line + 1`), or `[nopos]`.
// A symbol is its owners' names from the package down, joined by `.`, with `#<signature>` for a method
// (`Signature`'s parameter and result names); its flags are the source-level ones, in a fixed order.
//
// Usage: java -cp <scalac's class path>:<the compiled InitDump> dotty.tools.dotc.transform.init.InitDump <scalac's
// arguments>: the dump on stdout, scalac's diagnostics on stderr as scalac prints them.

package dotty.tools.dotc.transform.init

import dotty.tools.dotc.*
import core.*
import Contexts.*, Symbols.*, Flags.*, Types.*, Phases.*, Constants.*, StdNames.*, Decorators.*
import ast.tpd.*
import util.{SourceFile, SourcePosition}

import scala.collection.mutable

object InitDumpText:
  val flagNames: List[(FlagSet, String)] = List(
    Package -> "package", Module -> "module", Method -> "method", Param -> "param", ParamAccessor -> "paramAccessor",
    Accessor -> "accessor", CaseAccessor -> "caseAccessor", Mutable -> "mutable", Lazy -> "lazy", Deferred -> "deferred",
    Abstract -> "abstract", Trait -> "trait", Final -> "final", Sealed -> "sealed", Open -> "open", Case -> "case",
    Enum -> "enum", Private -> "private", Protected -> "protected", Local -> "local", Override -> "override",
    Inline -> "inline", Transparent -> "transparent", Macro -> "macro", Given -> "given", Implicit -> "implicit",
    Erased -> "erased", Synthetic -> "synthetic", Artifact -> "artifact", JavaDefined -> "java", Exported -> "exported")
  // `StableRealizable` is left out: the checker reads it nowhere (transform/init), and the typer sets it lazily, where a
  // path's realizability is checked (`SymDenotation.isRealizable`), which a tree does not record.

class InitDumpPhase(tag: String, after: String) extends Phase:
  def phaseName: String = "initDump" + tag.capitalize
  override val runsAfter: Set[String] = Set(after)

  private val sources = mutable.LinkedHashMap[SourceFile, Int]()

  protected def run(using Context): Unit =
    val unit = ctx.compilationUnit
    val text = printTree(unit.tpdTree, 0)

    val classes = mutable.ArrayBuffer[ClassSymbol]()
    new TreeTraverser:
      def traverse(t: Tree)(using Context): Unit =
        traverseChildren(t)
        t match
          case td: TypeDef if td.isClassDef => classes += td.symbol.asClass
          case _ =>
    .traverse(unit.tpdTree)

    import Util.*
    println(s"unit ${unit.source.file.path}")
    for c <- classes do
      println(s"class ${symId(c)} concrete=${isConcreteClass(c)} staticObject=${c.isStaticObject}")
    print(text)
  end run

  // The library members the evaluation follows (tests/init-view/library-members.txt, named by INITDUMP_MEMBERS): after
  // the pickler phase's units, each class's `defTree` (its root tree read: a class of the class path keeps its tree
  // under -Wsafe-init, `Symbols.retainsDefTree`), under `deftree <name>`, printed as a unit's tree is.
  override def runOn(units: List[CompilationUnit])(using Context): List[CompilationUnit] =
    println(s"phase $tag")
    val done = super.runOn(units)
    val members = sys.env.get("INITDUMP_MEMBERS").filter(f => f.nonEmpty && tag == "pickler")
    for file <- members; name <- scala.io.Source.fromFile(file).getLines().map(_.trim).filter(n => n.nonEmpty && !n.startsWith("#")) do
      val cls = getClassIfDefined(name)
      if !cls.exists || !cls.isClass then println(s"deftree $name missing")
      else
        cls.asClass.rootTree
        val tree = cls.defTree
        if tree.isEmpty then println(s"deftree $name empty")
        else
          println(s"deftree $name")
          print(printTree(tree, 1))
    done

  def symId(s: Symbol)(using Context): String =
    if !s.exists then "-"
    else if s.isRoot || s.isEmptyPackage || s.is(Package) then s.fullName.toString
    else
      val own = symId(s.owner)
      val sig =
        if s.is(Method) then
          val sg = s.signature
          "#(" + sg.paramsSig.map(_.toString).mkString(",") + ")" + sg.resSig.toString
        else ""
      own + "." + s.name.toString + sig

  def printTree(root: Tree, depth0: Int)(using Context): String =
    val out = new StringBuilder
    def line(s: String): Unit = out.append(s).append('\n')

    def sourceId(src: SourceFile): Int =
      sources.getOrElseUpdate(src, {
        val n = sources.size + 1
        val content = src.content()
        val has = content.nonEmpty
        val sha =
          if has then
            val md = java.security.MessageDigest.getInstance("SHA-256")
            md.update(new String(content).getBytes("UTF-8"))
            md.digest().map(b => f"$b%02x").mkString
          else "-"
        line(s"source s$n name=${src.file.name} path=${src.file.path} content=${if has then "yes" else "no"} sha256=$sha")
        n
      })

    def pos(t: Tree): String =
      if !t.span.exists then "[nopos]"
      else
        val p: SourcePosition = t.sourcePos
        if !p.exists then s"[nopos ${t.span.start}..${t.span.end}@${t.span.point}]"
        else
          val id = sourceId(p.source)
          s"[s$id ${t.span.start}..${t.span.end}@${t.span.point} ${p.startLine + 1}:${p.startColumn}-${p.endLine + 1}:${p.endColumn}]"

    def flags(s: Symbol): String =
      if !s.exists then "" else
        val fs = InitDumpText.flagNames.collect { case (f, n) if s.isOneOf(f) => n }
        if fs.isEmpty then "" else fs.mkString(" {", ",", "}")

    // The paths of a type that `Semantic.checkTermUsage` evaluates: each `TermRef` whose prefix is a singleton,
    // its children not visited, as its `TypeTraverser` does.
    def paths(tp: Type): List[String] =
      val found = mutable.ListBuffer[String]()
      new TypeTraverser:
        def traverse(tp: Type): Unit = tp match
          case tr @ TermRef(_: SingletonType, _) => found += symId(tr.symbol)
          case _ => traverseChildren(tp)
      .traverse(tp)
      found.toList

    def isTypeTree(t: Tree): Boolean =
      t.isInstanceOf[TypTree] || (t.isType && !t.isInstanceOf[MemberDef] && !t.isInstanceOf[Template])

    def facts(t: Tree): String = t match
      case Apply(fn, _) =>
        val margs = fn.tpe.widen match
          case mt: MethodType => mt.paramInfos.map { case _: ExprType => "=>"; case _ => "v" }.mkString("[", ",", "]")
          case other => "!" + other.getClass.getSimpleName
        s" fn=${symId(fn.symbol)} margs=$margs"
      case TypeApply(fn, _) => s" fn=${symId(fn.symbol)}"
      case sel @ Select(qual, _) =>
        val q = qual.tpe.widenSingleton.classSymbol
        val m = if sel.symbol.is(Method) || sel.tpe.widenSingleton.isInstanceOf[MethodicType] then " methodic" else ""
        s" sym=${symId(sel.symbol)}${flags(sel.symbol)} qual=${symId(q)}$m"
      case id: Ident =>
        val pre = id.tpe match
          case TermRef(NoPrefix, _) => "none"
          case TermRef(ThisType(tref), _) => "this(" + symId(tref.symbol) + ")"
          case TermRef(p: TermRef, _) => "term(" + symId(p.symbol) + ")"
          case TermRef(_, _) => "other"
          case _: ConstantType => "constant"
          case _ => "-"
        s" sym=${symId(id.symbol)}${flags(id.symbol)} pre=$pre"
      case This(_) => s" this=${symId(t.symbol)}"
      case Super(_, mix) => s" super=${symId(t.symbol)} mix=${if mix.isEmpty then "-" else mix.name.toString}"
      case n: New => s" new=${symId(Util.typeRefOf(n.tpe).symbol)}"
      case Literal(c) => s" const=${c.tag}:${c.value}"
      case Closure(_, meth, _) => s" meth=${symId(meth.symbol)}"
      case Return(_, from) => s" from=${symId(from.symbol)}"
      case Inlined(call, _, _) =>
        if call.isEmpty then " call=-" else s" call=${symId(call.symbol)} callpos=${pos(call)}"
      case b: Bind => s" sym=${symId(b.symbol)}${flags(b.symbol)}"
      case UnApply(fun, _, _) => s" fun=${symId(fun.symbol)}"
      case Typed(_, tpt) =>
        val unchecked = tpt.tpe match
          case AnnotatedType(_, annot) if annot.symbol == defn.UncheckedAnnot => " unchecked"
          case _ => ""
        unchecked
      case m: MemberDef => s" sym=${symId(m.symbol)}${flags(m.symbol)}"
      case _ => ""

    new TreeTraverser:
      var depth = depth0
      def traverse(t: Tree)(using Context): Unit =
        t match
          case EmptyTree => ()
          case t: ValDef if t eq EmptyValDef => ()
          case Thicket(Nil) => ()
          case _: Import | _: Export => ()
          case _ if isTypeTree(t) =>
            line("  " * depth + s"Tpt ${pos(t)} paths=${paths(t.tpe).mkString("[", ",", "]")}")
          case _ =>
            line("  " * depth + s"${t.getClass.getSimpleName} ${pos(t)}${facts(t)}")
            depth += 1
            traverseChildren(t)
            depth -= 1
    .traverse(root)
    out.toString
  end printTree

object InitDump:
  def main(args: Array[String]): Unit =
    val driver = new Driver:
      override def newCompiler(using Context): Compiler = new Compiler:
        override def phases: List[List[Phase]] =
          super.phases.flatMap { group =>
            val names = group.map(_.phaseName)
            if names.contains("pickler") then List(group, List(new InitDumpPhase("pickler", "pickler")))
            else if names.contains("initChecker") then List(group, List(new InitDumpPhase("initChecker", "initChecker")))
            else List(group)
          }
    val reporter = driver.process(args)
    System.out.flush()
    if reporter.hasErrors then sys.exit(1)
