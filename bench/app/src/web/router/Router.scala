package meridian.web.router

import meridian.core.effect.{Eff, Task}
import meridian.web.vdom.Node

/** A bidirectional path pattern: parses a list of segments into a value and prints it back. */
final class Path[A](val parse: List[String] => Option[(A, List[String])], val print: A => List[String]):
  def /[B, AB](that: Path[B])(using pc: PathConcat[A, B, AB]): Path[AB] =
    Path(
      segments => parse(segments).flatMap((a, rest) => that.parse(rest).map((b, tail) => (pc.join(a, b), tail))),
      ab =>
        val (a, b) = pc.split(ab)
        print(a) ++ that.print(b))
  def /(literal: String)(using pc: PathConcat[A, Unit, A]): Path[A] = this / Path.literal(literal)
  def xmap[B](to: A => B)(from: B => A): Path[B] = Path(segments => parse(segments).map((a, rest) => (to(a), rest)), b => print(from(b)))
  def option: Path[Option[A]] = Path(
    segments => parse(segments).map((a, rest) => (Some(a), rest)).orElse(Some((None, segments))),
    {
      case Some(a) => print(a)
      case None => Nil
    })
  def parseAll(segments: List[String]): Option[A] = parse(segments).collect { case (a, Nil) => a }
  def url(a: A): String = print(a).mkString("/", "/", "")

object Path:
  val root: Path[Unit] = Path(segments => Some(((), segments)), _ => Nil)
  def literal(text: String): Path[Unit] = Path(
    {
      case head :: rest if head == text => Some(((), rest))
      case _ => None
    },
    _ => List(text))
  val string: Path[String] = Path(
    {
      case head :: rest if head.nonEmpty => Some((head, rest))
      case _ => None
    },
    s => List(s))
  val int: Path[Int] = Path(
    {
      case head :: rest => head.toIntOption.map(i => (i, rest))
      case _ => None
    },
    i => List(i.toString))
  val long: Path[Long] = Path(
    {
      case head :: rest => head.toLongOption.map(l => (l, rest))
      case _ => None
    },
    l => List(l.toString))
  def const[A](a: A): Path[A] = Path(segments => Some((a, segments)), _ => Nil)

extension (literal: String)
  def /[B](that: Path[B]): Path[B] = Path.literal(literal) / that
  def /(next: String): Path[Unit] = Path.literal(literal) / Path.literal(next)

trait PathConcat[A, B, AB]:
  def split(ab: AB): (A, B)
  def join(a: A, b: B): AB
trait PathConcatLow2:
  given pair[A, B]: PathConcat[A, B, (A, B)] with
    def split(ab: (A, B)) = ab
    def join(a: A, b: B) = (a, b)
trait PathConcatLow1 extends PathConcatLow2:
  given unitLeft[B]: PathConcat[Unit, B, B] with
    def split(ab: B) = ((), ab)
    def join(a: Unit, b: B) = b
  given unitRight[A]: PathConcat[A, Unit, A] with
    def split(ab: A) = (ab, ())
    def join(a: A, b: Unit) = a
  given triple[A, B, C]: PathConcat[(A, B), C, (A, B, C)] with
    def split(ab: (A, B, C)) = ((ab._1, ab._2), ab._3)
    def join(a: (A, B), b: C) = (a._1, a._2, b)
object PathConcat extends PathConcatLow1:
  given units: PathConcat[Unit, Unit, Unit] with
    def split(ab: Unit) = ((), ())
    def join(a: Unit, b: Unit) = ()

/** A page type with the paths that lead to each of its cases, and the renderer of a page. */
final class Rule[Page](val parse: List[String] => Option[Page], val print: PartialFunction[Page, List[String]])
object Rule:
  def static[Page](path: Path[Unit], page: Page): Rule[Page] =
    new Rule(segments => path.parseAll(segments).map(_ => page), { case p if p == page => path.print(()) })
  def dynamic[Page, A](path: Path[A])(to: A => Page)(from: PartialFunction[Page, A]): Rule[Page] =
    new Rule(segments => path.parseAll(segments).map(to), from.andThen(path.print))

final class Router[Page](rules: List[Rule[Page]], notFound: Page, render: (Router[Page], Page) => Node):
  private var currentPage: Page = notFound
  private var history: List[Page] = Nil
  def parse(url: String): Page =
    val segments = url.split('/').toList.filter(_.nonEmpty)
    rules.iterator.map(_.parse(segments)).collectFirst { case Some(page) => page }.getOrElse(notFound)
  def urlFor(page: Page): String = rules.iterator.map(_.print.lift(page)).collectFirst { case Some(segments) => segments }.map(_.mkString("/", "/", "")).getOrElse("/404")
  def set(page: Page): Task[Unit] = Eff.succeed {
    history = currentPage :: history
    currentPage = page
  }
  def navigate(url: String): Task[Unit] = set(parse(url))
  def back: Task[Unit] = Eff.succeed {
    history match
      case previous :: rest =>
        currentPage = previous
        history = rest
      case Nil => ()
  }
  def page: Page = currentPage
  def node: Node = render(this, currentPage)
  def link(page: Page): String = urlFor(page)
  def visited: Int = history.length

object Router:
  def apply[Page](rules: List[Rule[Page]], notFound: Page)(render: (Router[Page], Page) => Node): Router[Page] =
    new Router(rules, notFound, render)
