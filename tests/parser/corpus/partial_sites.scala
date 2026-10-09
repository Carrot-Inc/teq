// Shapes of the partial-function sites of application code: collect, collectFirst, whenCase, catchSome.
import scala.util.matching.Regex

enum ServiceFlag:
  case Credits, Challenges, Volunteers
object ServiceFlag:
  def valueOfIgnoreCase(s: String): Option[ServiceFlag] = values.find(_.toString.equalsIgnoreCase(s))

enum ByDay:
  case Simple(day: Int)
  case Nth(n: Int, day: Int)
  def print: String = this match
    case Simple(d) => "Day" + d
    case Nth(n, d) => n + "xDay" + d

enum WaiverType:
  case Percentage(value: Int)
  case FixedCashOff(value: Int)
  case Free

enum ThemeGroup:
  case Colors(subgroup: String)
  case Fonts
  case Layout

enum Step:
  case Basics, Trigger, Action, Review

sealed trait ButtonOpt
sealed trait ButtonTone extends ButtonOpt
case object Primary extends ButtonTone
case object Danger extends ButtonTone
sealed trait ButtonScale extends ButtonOpt
case object Sm extends ButtonScale
case object Md extends ButtonScale
case class IconBefore(icon: String) extends ButtonOpt
case class IconAfter(icon: String) extends ButtonOpt

sealed trait Validated[+A]
case class Valid[A](value: A) extends Validated[A]
case class Invalid(errors: List[String]) extends Validated[Nothing]

sealed trait Action
case class WaiverCode(codeType: Option[CodeType]) extends Action
case object SendNotice extends Action
sealed trait CodeType
case class GeneralWaiver(code: String) extends CodeType
case object UniqueCode extends CodeType

sealed trait AppError
case class JsException(code: Int, message: String) extends AppError
case class Timeout(ms: Int) extends AppError
case class PositionError(code: Int, message: String)
case class Position(lat: Double, lng: Double)

final case class IO[+E, +A](run: () => Either[E, A]):
  def map[B](f: A => B): IO[E, B] = IO(() => run().map(f))
  def asRight: IO[E, Either[Nothing, A]] = map(a => Right(a))
  def catchSome[E1 >: E, A1 >: A](pf: PartialFunction[E, IO[E1, A1]]): IO[E1, A1] =
    IO(() =>
      run() match
        case Left(e) => pf.applyOrElse(e, (e1: E) => IO.fail(e1)).run()
        case Right(a) => Right(a)
    )

object IO:
  def succeed[A](a: A): IO[Nothing, A] = IO(() => Right(a))
  def fail[E](e: E): IO[E, Nothing] = IO(() => Left(e))
  def whenCase[E, A, B](a: => A)(pf: PartialFunction[A, IO[E, B]]): IO[E, Option[B]] =
    val value = a
    if pf.isDefinedAt(value) then pf(value).map(b => Some(b)) else IO.succeed(None)
  def whenCaseDiscard[E, A](a: => A)(pf: PartialFunction[A, IO[E, Any]]): IO[E, Unit] =
    whenCase(a)(pf).map(_ => ())

def themedButton(opts: ButtonOpt*): String =
  val intent = opts.collectFirst { case i: ButtonTone => i }.getOrElse(Primary)
  val size = opts.collectFirst { case s: ButtonScale => s }.getOrElse(Md)
  val leading = opts.collectFirst { case IconBefore(icon) => icon }
  val trailing = opts.collectFirst { case IconAfter(icon) => icon }
  s"$intent/$size/$leading/$trailing"

def checkStep(s: Step, filled: Set[Step]): Validated[Step] =
  if filled.contains(s) then Valid(s) else Invalid(List(s.toString + " missing"))

def firstMissingStepBetween(from: Step, until: Step, filled: Set[Step]): Option[Step] =
  Step.values.toList
    .filter(s => s.ordinal >= from.ordinal && s.ordinal < until.ordinal)
    .collectFirst { case s if checkStep(s, filled).isInstanceOf[Invalid] => s }

val agentRules: List[(Regex, String)] = List(
  ("Firefox/".r, "Firefox"),
  ("Edg/".r, "Edge"),
  ("Chrome/".r, "Chrome")
)

def parseAgent(ua: String): Option[String] =
  agentRules.collectFirst { case (regex, name) if regex.findFirstIn(ua).isDefined => name }

def duplicateNotes(tokens: List[(String, List[String])]): List[String] =
  tokens
    .groupBy((token, key) => (key, token))
    .values
    .toList
    .collect:
      case group if group.sizeIs > 1 =>
        group.map((token, _) => token).distinct match
          case raw :: Nil => s"Duplicate class '$raw'."
          case raws => s"${raws.mkString(" and ")} are the same class."
    .sorted

def doubledVariant(bare: List[String]): Option[String] =
  bare
    .groupBy(identity)
    .toList
    .sortBy(_._1)
    .collectFirst:
      case (variant, occurrences) if occurrences.sizeIs > 1 =>
        s"variant '$variant:' is repeated."

@main def main(): Unit =
  // a List of pairs mapped and then narrowed to the known keys
  val flagDrafts = List("credits" -> true, "unknown" -> false, "VOLUNTEERS" -> true)
  val flagUpdates = flagDrafts
    .map { case (key, value) => (ServiceFlag.valueOfIgnoreCase(key), value) }
    .collect { case (Some(key), value) => (key, value) }
  println(flagUpdates)

  // type test on enum cases in a Set, then sorted
  val byDay: Set[ByDay] = Set(ByDay.Nth(2, 1), ByDay.Simple(3), ByDay.Simple(1))
  val daysOfWeek = byDay.collect { case simple: ByDay.Simple => simple }.toList.sortBy(_.day).map(_.print.take(4))
  println(daysOfWeek)

  // Option.collect with a qualified case pattern
  val initial: Option[(Int, WaiverType)] = Some((1, WaiverType.Percentage(15)))
  println(initial.map(_._2).collect { case WaiverType.Percentage(value) => value })
  println(initial.map(_._2).collect { case WaiverType.FixedCashOff(value) => value })

  // Map.collect on optional values followed by filter
  val trails = Map("a" -> List(3, 9, 4), "b" -> List[Int](), "c" -> List(1))
  val recent = trails
    .map((k, v) => (k, v.maxOption))
    .collect { case (k, Some(v)) => k -> v }
    .filter(_._2 >= 2)
  println(recent)

  // nested pattern over the values of a Map
  val drafts: Map[Int, Option[Validated[String]]] = Map(1 -> Some(Valid("north")), 2 -> None, 3 -> Some(Invalid(List("x"))))
  val changedWings = drafts.values.map(identity).collect { case Some(Valid(zone)) => zone }.toList
  println(changedWings)

  // collectFirst over varargs
  println(themedButton(Danger, IconBefore("plus"), Sm))
  println(themedButton())

  // collectFirst over enum values with a guard
  println(firstMissingStepBetween(Step.Basics, Step.Review, Set(Step.Basics, Step.Action)))
  println(firstMissingStepBetween(Step.Basics, Step.Review, Set(Step.Basics, Step.Trigger, Step.Action)))

  // guard that runs a regex
  println(parseAgent("Mozilla/5.0 Chrome/120 Edg/120"))
  println(parseAgent("Mozilla/5.0 Gecko Firefox/121"))
  println(parseAgent("curl/8"))

  // Option.collect inside flatMap, then flatten
  val action: Option[Action] = Some(WaiverCode(Some(GeneralWaiver("WAIVE10"))))
  val maybeGeneralWaiver = action
    .collect { case WaiverCode(codeType) => codeType }
    .flatMap(_.collect { case general: GeneralWaiver => general.code })
  println(maybeGeneralWaiver)
  println(Option[Action](SendNotice).collect { case WaiverCode(codeType) => codeType })

  // collect on a grouped Map with the indented form
  val settings = List("bg" -> ThemeGroup.Colors("brand"), "fg" -> ThemeGroup.Colors("brand"), "font" -> ThemeGroup.Fonts, "pad" -> ThemeGroup.Layout)
  val colors = settings
    .groupBy(_._2)
    .collect:
      case (ThemeGroup.Colors(subgroup), entries) => (subgroup, entries.map(_._1))
    .toList
    .sortBy(_._1)
  println(colors)

  // collect and collectFirst with multi-line bodies
  println(duplicateNotes(List(("p-4", List("md")), ("p-4", List("md")), ("m-2", Nil), ("m-2", Nil), ("m-2", Nil), ("w-1", Nil))))
  println(doubledVariant(List("md", "hover", "md")))
  println(doubledVariant(List("md", "hover")))

  // whenCase over an Either
  val flags: Either[String, List[ServiceFlag]] = Right(List(ServiceFlag.Credits))
  val handled = IO.whenCase(flags) {
    case Right(value) => IO.succeed("flags: " + value.mkString(","))
    case Left(failure) => IO.succeed("failed: " + failure)
  }
  println(handled.run())
  val skipped = IO.whenCase(Option.empty[Int]) {
    case Some(n) => IO.succeed(n)
  }
  println(skipped.run())
  println(IO.whenCaseDiscard(Some(1)) { case Some(n) => IO.succeed(n) }.run())

  // catchSome with a type test and a guard, other errors stay failures
  def locate(error: Option[AppError]): IO[AppError, Either[PositionError, Position]] =
    val fetched: IO[AppError, Position] = error match
      case Some(e) => IO.fail(e)
      case None => IO.succeed(Position(1.5, 2.5))
    fetched
      .asRight
      .catchSome {
        case e: JsException if e.code != 0 =>
          IO.succeed(Left(PositionError(code = e.code, message = e.message)))
      }
  println(locate(None).run())
  println(locate(Some(JsException(1, "denied"))).run())
  println(locate(Some(JsException(0, "unknown"))).run())
  println(locate(Some(Timeout(3000))).run())
