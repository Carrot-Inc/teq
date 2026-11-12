// The kinds of the std's classes and objects against scala-library's (tests/std-kinds.sh): each line of the
// file given, `name<TAB>kind<TAB>sealed|-<TAB>final|-<TAB>case|-` (teq's `TEQ_STD_KINDS` listing, the names
// scala-library's), is looked up as scalac 3.8.4 resolves the name on its own class path, and printed as a
// result of the suite when both sides declare a class or an object under it:
//   pass <name>                                  the kind and `sealed`, `final`, `case` are the library's
//   fail <name><TAB><the std's>, <the library's> they are not
// An object's name ends in `$`, as its class's does. A line is no result when the std's side is no declared
// class or object (an enum, an opaque type, a class the compiler builds in), when the library has nothing under
// the name or no class (a type member, an alias), and when the library's class is not scala-library's own (the
// JDK's, read from the JDK): those are outside what the suite compares.
import dotty.tools.dotc.Compiler
import dotty.tools.dotc.core.Contexts.*
import dotty.tools.dotc.core.Flags.*
import dotty.tools.dotc.core.Names.*
import dotty.tools.dotc.core.Symbols.*
import dotty.tools.dotc.core.Decorators.*
import scala.io.Source

// The symbol a qualified name stands for: a package, a class, or (`X$`) an object's class.
def owner(path: String)(using Context): Symbol =
  path.split('.').foldLeft(defn.RootClass: Symbol) { (at, seg) =>
    if !at.exists then at
    else if seg.endsWith("$") then at.info.member(seg.dropRight(1).toTermName).symbol.moduleClass
    else
      val pkg = at.info.member(seg.toTermName).symbol
      if pkg.is(Package) then pkg.moduleClass else at.info.member(seg.toTypeName).symbol
  }

// A class's kind and flags in the listing's words. An object is final by what it is, on both sides.
def described(sym: Symbol)(using Context): String =
  val kind =
    if sym.is(Module) then "object"
    else if sym.is(Trait) then "trait"
    else if sym.is(Abstract) then "abstract class"
    else "class"
  val flags = Seq(
    Option.when(sym.is(Sealed))("sealed"),
    Option.when(sym.is(Final) || sym.is(Module))("final"),
    Option.when(sym.is(Case))("case"),
  ).flatten
  (kind +: flags).mkString(" ")

@main def kinds(file: String): Unit =
  val base = new ContextBase
  val c0 = base.initialCtx.fresh
  c0.setSetting(c0.settings.classpath, System.getProperty("java.class.path"))
  base.initialize()(using c0)
  val run = new Compiler().newRun(using c0)
  inContext(run.runContext) {
    for line <- Source.fromFile(file).getLines() if line.nonEmpty do
      val Array(name, kind, flags*) = line.split('\t'): @unchecked
      if Set("trait", "abstract class", "class", "object")(kind) then
        val bare = name.stripSuffix("$")
        val test = if kind == "object" then bare + "$" else bare
        val sym = try owner(test) catch case _: Throwable => NoSymbol
        // scala-library's own: a Scala class read from its jar.
        val library = sym.exists && sym.isClass && !sym.is(JavaDefined) && {
          sym.ensureCompleted()
          val from = sym.associatedFile
          from != null && from.underlyingSource.exists(_.name.startsWith("scala-library"))
        }
        if library then
          val std = (kind +: flags.filter(_ != "-")).mkString(" ")
          val theirs = described(sym)
          if std == theirs then println(s"pass $test")
          else println(s"fail $test\t$std in the std, $theirs in scala-library")
  }
