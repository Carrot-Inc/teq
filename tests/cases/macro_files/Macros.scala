package twlib

import java.nio.file.{Files, Path}
import java.util.concurrent.ConcurrentHashMap
import scala.annotation.tailrec
import scala.jdk.CollectionConverters.*
import scala.quoted.*

object Tailwind:
  opaque type Tw = String
  object Tw:
    val empty: Tw = ""
    extension (classes: Tw)
      def raw: String = classes
      def ++(other: Tw): Tw = if classes.isEmpty then other else if other.isEmpty then classes else s"$classes $other"

  extension (inline sc: StringContext)
    inline def tw(inline args: Any*): Tw = ${ Macros.twImpl('sc, 'args) }

  inline def classNames(inline classes: (String | (String, Boolean) | Tw | (Tw, Boolean))*): Tw =
    ${ Macros.classListImpl('classes) }

  def concatClasses(classes: Seq[String | (String, Boolean) | Tw | (Tw, Boolean)]): Tw =
    classes
      .flatMap:
        case (name: String, cond: Boolean) => Option.when(cond)(name)
        case name: String                  => Some(name)
      .filter(_.nonEmpty)
      .mkString(" ")

final case class ClassSet(classes: Map[String, Option[Int]], variants: Set[String], signatures: Map[Int, String])

object ClassSet:
  def parse(file: Path): ClassSet =
    val lines = Files.readAllLines(file).asScala.toList.filter(line => line.nonEmpty && !line.startsWith("#"))
    def entries(prefix: String): List[String] = lines.filter(_.startsWith(prefix)).map(_.drop(prefix.length))
    def keyed(prefix: String): List[(String, String)] =
      entries(prefix).map: entry =>
        entry.indexOf('|') match
          case -1 => (entry, "")
          case i  => (entry.take(i), entry.drop(i + 1))
    ClassSet(
      classes = keyed("class:").map((name, sig) => (name, Option.when(sig.nonEmpty)(sig.toInt))).toMap,
      variants = entries("variant:").toSet,
      signatures = keyed("sig:").map((id, props) => (id.toInt, props)).toMap
    )

object Loader:
  val FileName = "classes.txt"
  private val cache = new ConcurrentHashMap[Path, ClassSet]
  private val closestFile = new ConcurrentHashMap[Path, Path]

  def near(sourcePath: Path): Option[ClassSet] =
    Option(sourcePath.getParent).flatMap(nearest).map(load)

  private def nearest(sourceDir: Path): Option[Path] =
    Option(closestFile.get(sourceDir)).orElse:
      val found = LazyList
        .iterate(Option(sourceDir))(_.flatMap(dir => Option(dir.getParent)))
        .takeWhile(_.isDefined)
        .flatten
        .map(_.resolve(FileName))
        .find(Files.exists(_))
      found.foreach(closestFile.put(sourceDir, _))
      found

  private def load(file: Path): ClassSet =
    val mtime = Files.getLastModifiedTime(file)
    cache.compute(file, (_, previous) => Option(previous).getOrElse(ClassSet.parse(file)))

final case class StaticArg[P](value: String, pos: P, conditional: Boolean)

object Macros:
  import Tailwind.Tw

  final case class Token(raw: String, variants: List[String], base: String)

  def parseAll(value: String): List[Token] =
    value.split("\\s+").toList.filter(_.nonEmpty).map: token =>
      token.split(":").toList.reverse match
        case base :: variantsReversed => Token(token, variantsReversed.reverse, base)
        case Nil                      => Token(token, Nil, token)

  def errors(tokens: List[Token], classSet: ClassSet): List[String] =
    val unknown = tokens.flatMap: token =>
      token.variants.find(v => !classSet.variants(v)).map(v => s"unknown variant '$v' in '${token.raw}'")
        .orElse(Option.unless(classSet.classes.contains(token.base))(s"unknown class '${token.base}'"))
    val conflicts = tokens
      .flatMap(token => classSet.classes.get(token.base).flatten.map(sig => (token, sig)))
      .groupBy((token, sig) => (sig, token.variants))
      .values
      .toList
      .sortBy(_.head._1.raw)
      .collect:
        case (first, sig) :: (second, _) :: _ if first.variants.isEmpty =>
          s"'${first.raw}' and '${second.raw}' conflict: both set {${classSet.signatures.getOrElse(sig, "?")}}"
    unknown ++ conflicts

  def usingClassSet(using Quotes)(pos: quotes.reflect.Position)(check: ClassSet => Unit): Unit =
    import quotes.reflect.*
    pos.sourceFile.getJPath match
      case None => ()
      case Some(sourcePath) =>
        Loader.near(sourcePath) match
          case Some(classSet) => check(classSet)
          case None => report.error(s"no ${Loader.FileName} above ${pos.sourceFile.path}", pos)

  def reportAll(using Quotes)(messages: List[String], pos: quotes.reflect.Position): Unit =
    import quotes.reflect.*
    if messages.nonEmpty then report.error(messages.distinct.mkString("\n"), pos)

  def twImpl(sc: Expr[StringContext], args: Expr[Seq[Any]])(using Quotes): Expr[String] =
    import quotes.reflect.*
    args match
      case Varargs(interpolated) if interpolated.nonEmpty => report.errorAndAbort("no interpolation", Position.ofMacroExpansion)
      case _ => ()
    val value = StringContext.processEscapes(sc.valueOrAbort.parts.headOption.getOrElse(""))
    usingClassSet(Position.ofMacroExpansion): classSet =>
      reportAll(errors(parseAll(value), classSet), Position.ofMacroExpansion)
    Expr(value)

  def classListImpl(classes: Expr[Seq[String | (String, Boolean) | Tw | (Tw, Boolean)]])(using Quotes): Expr[Tw] =
    import quotes.reflect.*
    val twType = TypeRepr.of[Tw]

    @tailrec
    def outerStripped(term: Term): Term = term match
      case Typed(inner, _)                               => outerStripped(inner)
      case Block(Nil, inner)                             => outerStripped(inner)
      case TypeApply(Select(inner, "$asInstanceOf$"), _) => outerStripped(inner)
      case _                                             => term

    @tailrec
    def stripped(term: Term): Term = outerStripped(term) match
      case Inlined(_, _, inner) => stripped(inner)
      case other                => other

    def tupleArg(term: Term): Option[(Term, Term)] = term match
      case Apply(TypeApply(Select(tuple, "apply"), _), List(a, b)) if tuple.symbol.fullName == "scala.Tuple2" => Some((a, b))
      case Apply(Select(tuple, "apply"), List(a, b)) if tuple.symbol.fullName == "scala.Tuple2"               => Some((a, b))
      case Apply(TypeApply(Select(arrowAssoc, "->"), _), List(b)) =>
        stripped(arrowAssoc) match
          case Apply(_, List(a)) => Some((a, b))
          case _                 => None
      case _ => None

    def isSpliced(term: Term): Boolean =
      val finder = new TreeAccumulator[Boolean]:
        def foldTree(found: Boolean, tree: Tree)(owner: Symbol): Boolean =
          found || (tree match
            case Apply(Select(sc, "apply"), _) if sc.symbol.fullName == "scala.StringContext" => true
            case _                                                                            => foldOverTree(found, tree)(owner))
      finder.foldTree(false, term)(Symbol.spliceOwner)

    @tailrec
    def isTw(term: Term): Boolean = outerStripped(term) match
      case Inlined(Some(call: Term), _, _) => call.tpe <:< twType
      case Inlined(_, _, inner)            => isTw(inner)
      case other                           => other.tpe <:< twType

    @tailrec
    def classify(term: Term, conditional: Boolean): Option[StaticArg[Position]] =
      if isTw(term) then None
      else
        stripped(term) match
          case literal @ Literal(StringConstant(value)) => Some(StaticArg(value, literal.pos, conditional))
          case other =>
            tupleArg(other) match
              case Some((a, _)) if !conditional => classify(a, conditional = true)
              case _ =>
                report.error(if isSpliced(other) then "interpolated" else "dynamic", other.pos)
                None

    val args = stripped(classes.asTerm) match
      case Repeated(elements, _) => elements
      case _ => report.errorAndAbort("no spread", classes.asTerm.pos)
    usingClassSet(Position.ofMacroExpansion): classSet =>
      val statics = args.flatMap(classify(_, conditional = false))
      val unconditional = statics.filter(!_.conditional).flatMap(s => parseAll(s.value))
      statics.foreach(s => reportAll(errors(parseAll(s.value), classSet).filter(_.startsWith("unknown")), s.pos))
      reportAll(errors(unconditional, classSet), Position.ofMacroExpansion)
    '{ Tailwind.concatClasses($classes) }
