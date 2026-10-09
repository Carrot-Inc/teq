// The lean std's selections that scala-library 3.8.4 resolves: each key of the file given first,
// `owner<TAB>name<TAB>shape`, resolved against the
// class path given second (scalajs-library) and the JVM's (scala-library, the JDK) as scalac's
// TreeUnpickler resolves the tree teq writes for it. Prints the keys that resolve; with
// `--refused` the others, each with what the library has under the name; with
// `--inverse=<file>` also each key of that file, scala-library's of the inverse table, after
// `inverse` and before `resolves` or what the library has. A refused key whose member the
// library has as a Java varargs method of the key's parameters and the varargs, of the key's
// result, is printed after `varargs` with that method's shape and the element's class (the
// inverse table's appended form).
//   shape `object`   TERMREF: the object `name` of the package or class `owner`
//   shape `-:r`      SELECT: `owner`'s member `name` at `NotAMethod`, of the result `r`
//   shape `p,..:r`   SELECTin: `name` declared in `owner`, or inherited by it, at the signature
//                    of the parameters `p` (`[n]` a type parameter clause) and the result `r`
// `owner` is a qualified name, `X$` an object's class, `name` a member's name or its target
// name after a `/`.
import dotty.tools.dotc.Compiler
import dotty.tools.dotc.core.Contexts.*
import dotty.tools.dotc.core.Denotations.*
import dotty.tools.dotc.core.Flags.*
import dotty.tools.dotc.core.Names.*
import dotty.tools.dotc.core.NameKinds
import dotty.tools.dotc.core.Signature
import dotty.tools.dotc.core.SourceLanguage
import dotty.tools.dotc.core.Symbols.*
import dotty.tools.dotc.core.TypeErasure
import dotty.tools.dotc.core.Decorators.*
import scala.io.Source

@main def check(keys: String, classpath: String, mode: String*): Unit =
  val base = new ContextBase
  val c0 = base.initialCtx.fresh
  c0.setSetting(c0.settings.classpath, s"$classpath${java.io.File.pathSeparator}${System.getProperty("java.class.path")}")
  base.initialize()(using c0)
  val run = new Compiler().newRun(using c0)
  val refused = mode.contains("--refused")
  val inverse = mode.collect { case m if m.startsWith("--inverse=") => m.stripPrefix("--inverse=") }
  inContext(run.runContext) {
    def found(line: String) =
      val Array(owner, name, shape) = line.split('\t')
      try resolve(owner, name, shape) catch case t: Throwable => Left(s"${t.getClass.getSimpleName}: ${t.getMessage}")
    for line <- Source.fromFile(keys).getLines() if line.nonEmpty do
      found(line) match
        case Right(()) => if !refused then println(line)
        case Left(why) =>
          if refused then println(s"$line\t$why")
          else
            val Array(owner, name, shape) = line.split('\t')
            for (libraryShape, elem) <- javaVarargs(owner, name, shape) do println(s"varargs\t$line\t$libraryShape\t$elem")
    for file <- inverse; line <- Source.fromFile(file).getLines() if line.nonEmpty do
      println(s"inverse\t$line\t${found(line).fold(identity, _ => "resolves")}")
  }

def sigText(sig: Signature)(using Context): String =
  if sig == Signature.NotAMethod then "-"
  else
    val params = sig.paramsSig.map {
      case n: Int => s"[$n]"
      case n => n.toString
    }
    s"${params.mkString(",")}:${sig.resSig}"

def owner(path: String)(using Context): Symbol =
  path.split('.').foldLeft(defn.RootClass: Symbol) { (at, seg) =>
    if !at.exists then at
    else if seg.endsWith("$") then at.info.member(seg.dropRight(1).toTermName).symbol.moduleClass
    else
      val pkg = at.info.member(seg.toTermName).symbol
      if pkg.is(Package) then pkg.moduleClass else at.info.member(seg.toTypeName).symbol
  }

def resolve(path: String, member: String, shape: String)(using Context): Either[String, Unit] =
  val cls = owner(path)
  if !cls.exists then return Left("no such owner")
  val (simple, target) = member.split('/') match
    case Array(n, t) => (n, t)
    case _ => (member, member)
  // A default getter's name is derived (`DefaultGetterName`), not the simple name it prints as.
  def termName(text: String): TermName = text match
    case s"$base$$default$$$n" if n.nonEmpty && n.forall(_.isDigit) => NameKinds.DefaultGetterName(base.toTermName, n.toInt - 1)
    case _ => text.toTermName
  val name = termName(simple)
  val targetName = termName(target)
  def alternatives(d: Denotation) = d.alternatives.map(a => s"${sigText(a.signature)}${if a.symbol.targetName != a.symbol.name then "/" + a.symbol.targetName else ""}").mkString(" ")
  shape match
    case "object" =>
      val m = cls.info.member(name).symbol
      if m.is(Module) then Right(()) else Left("no such object")
    case byName if byName.startsWith("-:") =>
      val d = cls.thisType.findMember(name, cls.thisType)
      val at = d.alternatives.filter(a => a.signature == Signature.NotAMethod && a.symbol.hasTargetName(targetName))
      val result = at.map(a => TypeErasure.sigName(a.info.finalResultType, SourceLanguage(a.symbol)).toString)
      if at.size == 1 && result == List(byName.drop(2)) then Right(()) else Left(s"has ${alternatives(d)} of ${result.mkString(" ")}")
    case sig =>
      def matching(d: Denotation) = d.alternatives.filter(a => sigText(a.signature) == sig && a.symbol.hasTargetName(targetName))
      val declared = cls.typeRef.decl(name)
      val inherited = cls.asClass.classDenot.findMember(name, cls.thisType, EmptyFlags, excluded = Private)
      if matching(declared).size == 1 || matching(declared).isEmpty && matching(inherited).size == 1 then Right(())
      else Left(s"has ${alternatives(declared)} declared, ${alternatives(inherited)} inherited")

def javaVarargs(path: String, member: String, shape: String)(using Context): Option[(String, String)] =
  val cls = owner(path)
  val cut = shape.lastIndexOf(':')
  if !cls.isClass || cut < 0 || shape.startsWith("-:") || member.contains('/') then return None
  val (params, result) = (shape.take(cut), shape.drop(cut + 1))
  val d = cls.asClass.classDenot.findMember(member.toTermName, cls.thisType, EmptyFlags, excluded = Private)
  val found = d.alternatives.flatMap { a =>
    a.info.paramInfoss.flatten.lastOption.filter(t => a.symbol.is(JavaDefined) && t.isRepeatedParam).map { last =>
      val elem = TypeErasure.sigName(last.argInfos.head, SourceLanguage.Java).toString
      (sigText(a.signature), elem)
    }
  }.filter { (text, elem) => text == s"${if params.isEmpty then "" else params + ","}$elem[]:$result" }
  found match
    case List(one) => Some(one)
    case _ => None
