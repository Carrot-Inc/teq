// The reflection API's inverse: each key of the lean
// std's `scala.quoted.Reflect` of the file given first (`owner<TAB>name<TAB>shape`, as
// TEQ_STD_SHAPES writes them), resolved against scala-library 3.8.4's `Quotes#reflectModule`,
// which a pickle reaches through a `Quotes` value's `reflect`. Prints per key the accessor of
// `reflectModule` the owner stands for, the class that declares the member there and the
// member's signature, or with `--refused` the keys without one and why:
//   `Reflect$.X$` (an object, `XMethods`, `XTypeTest`)  the accessor `X`, a val or a given
//                                                        whose type's class has the member
//   `Reflect$.X` (a class: `TreeMap`)                    the trait `X` of `reflectModule`
//   `Reflect$` (the object itself)                      `reflectModule` (`asTerm`) or, for
//                                                        shape `object`, the accessor itself
// A teq type of the API in a shape (`scala.quoted.Reflect$.Symbol`) stands for scala-library's
// abstract type, `java.lang.Object` erased, or for a trait of `reflectModule` by its name.
import dotty.tools.dotc.Compiler
import dotty.tools.dotc.core.Contexts.*
import dotty.tools.dotc.core.Denotations.*
import dotty.tools.dotc.core.Flags.*
import dotty.tools.dotc.core.Names.*
import dotty.tools.dotc.core.Signature
import dotty.tools.dotc.core.Symbols.*
import dotty.tools.dotc.core.Types.*
import dotty.tools.dotc.core.Decorators.*
import scala.io.Source

@main def reflect(keys: String, mode: String*): Unit =
  val base = new ContextBase
  val c0 = base.initialCtx.fresh
  c0.setSetting(c0.settings.classpath, System.getProperty("java.class.path"))
  base.initialize()(using c0)
  val run = new Compiler().newRun(using c0)
  val refused = mode.contains("--refused")
  // `--results`: scala-library's declared result of each member, unerased, for the std's to be
  // held to (an abstract type of the API by its simple name).
  val results = mode.contains("--results")
  inContext(run.runContext) {
    val quotes = requiredClass("scala.quoted.Quotes")
    val module = quotes.info.member("reflectModule".toTypeName).symbol.asClass
    for line <- Source.fromFile(keys).getLines() if line.startsWith("scala.quoted.Reflect$") do
      val Array(owner, name, shape) = line.split('\t')
      (try resolve(module, owner, name, shape) catch case t: Throwable => Left(s"${t.getClass.getSimpleName}: ${t.getMessage}")) match
        case Right(found) if results => println(s"$line\t${declaredResult(module, found, name)}")
        case Right(found) => if !refused then println(s"$line\t$found")
        case Left(why) => if refused then println(s"$line\t$why")
  }

def sigText(sig: Signature)(using Context): String =
  if sig == Signature.NotAMethod then "-"
  else
    val params = sig.paramsSig.map {
      case n: Int => s"[$n]"
      case n => n.toString
    }
    s"${params.mkString(",")}:${sig.resSig}"

// A teq shape's types in scala-library's erasure: the API's types are abstract there, or the
// traits of `reflectModule`, named by its path.
def mapped(module: ClassSymbol, shape: String)(using Context): String =
  shape.split("(?=[,:])|(?<=[,:])").map { part =>
    if part.startsWith("scala.quoted.Reflect$.") then
      val n = part.stripPrefix("scala.quoted.Reflect$.")
      val t = module.info.member(n.toTypeName).symbol
      if t.isClass then t.fullName.toString.replace("$", ".") else "java.lang.Object"
    else part
  }.mkString

// The accessor of `reflectModule` that `owner` stands for and the class that declares its
// members, or the trait itself.
def ownerOf(module: ClassSymbol, owner: String)(using Context): Either[String, (String, ClassSymbol)] =
  owner.stripPrefix("scala.quoted.Reflect$") match
    case "" => Right(("-", module))
    case rest if rest.startsWith(".") && rest.endsWith("$") && rest.drop(1).split('.').forall(_.endsWith("$")) =>
      // `Printer$.TreeCode$`: the accessors in turn, `Printer` then its `TreeCode`.
      val names = rest.drop(1).split('.').map(_.dropRight(1)).toList
      names.foldLeft[Either[String, ClassSymbol]](Right(module)) { (at, n) =>
        at.flatMap { c =>
          val accessor = c.info.member(n.toTermName)
          if !accessor.exists then Left(s"${c.name} has no $n")
          else accessor.info.finalResultType.classSymbol match
            case k: ClassSymbol => Right(k)
            case _ => Left(s"${c.name}'s $n is of no class")
        }
      }.map(c => (names.mkString("."), c))
    case rest if rest.startsWith(".") && !rest.drop(1).contains('$') =>
      val n = rest.drop(1)
      module.info.member(n.toTypeName).symbol match
        case c: ClassSymbol => Right((s"class:$n", c))
        case _ => Left(s"reflectModule has no trait $n")
    case rest => Left(s"no owner of reflectModule for $rest")

def resolve(module: ClassSymbol, owner: String, name: String, shape: String)(using Context): Either[String, String] =
  own(module, owner, name, shape).left.flatMap(why => extension(module, owner, name, shape).left.map(_ => why))

// A member of a class of teq's std that scala-library declares on its abstract type as an
// extension of `XMethods` (`Constant.value`, `ConstantMethods.value(c)`): the receiver its
// first parameter, the accessor marked `ext:`.
def extension(module: ClassSymbol, owner: String, name: String, shape: String)(using Context): Either[String, String] =
  val cls = owner.stripPrefix("scala.quoted.Reflect$.")
  if cls.contains('$') || cls.contains('.') || shape == "object" || name == "<init>" then return Left("no extension")
  val withRecv = shape.split(':') match
    case Array(params, result) =>
      val ps = params.split(',').filter(_.nonEmpty)
      val (tps, rest) = ps.span(_.startsWith("["))
      (tps ++ Array("java.lang.Object") ++ rest).mkString(",") + ":" + result
    case _ => return Left("no extension")
  own(module, s"scala.quoted.Reflect$$.${cls}Methods$$", name, withRecv).map(found => s"ext:$found")

def own(module: ClassSymbol, owner: String, name: String, shape: String)(using Context): Either[String, String] =
  ownerOf(module, owner).flatMap { (accessor, cls) =>
    val n = name.toTermName
    shape match
      case "object" =>
        val m = cls.info.member(n)
        if !m.exists then Left(s"${cls.name} has no $name")
        else m.info.finalResultType.classSymbol match
          case c: ClassSymbol => Right(s"$name\t${c.fullName}\tobject")
          case _ => Left(s"${cls.name}'s $name is of no class")
      case sig =>
        val wanted = mapped(module, sig)
        val d = cls.thisType.member(n)
        val alts = d.alternatives
        val bySig = alts.filter(a => sigText(a.signature) == wanted)
        val arity = wanted.split(':').head.split(',').count(_.nonEmpty)
        val byArity = alts.filter(a => sigText(a.signature).split(':').head.split(',').count(_.nonEmpty) == arity)
        val pick = if bySig.size == 1 then bySig else if alts.size == 1 then alts else byArity
        pick match
          case List(a) => Right(s"$accessor\t${a.symbol.owner.fullName}\t${sigText(a.signature)}")
          case Nil => Left(s"${cls.name} has no $name of $wanted (has ${alts.map(a => sigText(a.signature)).mkString(" ")})")
          case many => Left(s"${cls.name} has ${many.size} $name of $wanted")
  }

// The declared result of the member a row names, its abstract types of the API by name.
def declaredResult(module: ClassSymbol, found: String, name: String)(using Context): String =
  val Array(accessor, owner, sig) = found.split('\t')
  val cls = if owner.startsWith("scala.quoted.Quotes.reflectModule") then
    val n = owner.stripPrefix("scala.quoted.Quotes.reflectModule").stripPrefix(".")
    if n.isEmpty then module else module.info.member(n.toTypeName).symbol
  else requiredClass(owner)
  val alts = cls.info.member(name.toTermName).alternatives.filter(a => sigText(a.signature) == sig)
  alts.headOption.map(_.info.finalResultType.show).getOrElse("?")
