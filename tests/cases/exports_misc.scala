package demo

import Api.{*, given}
import Instances.combineAll

trait Semigroup[A]:
  extension (a: A) def combine(b: A): A

object Instances:
  given Semigroup[Int] with
    extension (a: Int) def combine(b: Int): Int = a + b

  given concat: Semigroup[String] with
    extension (a: String) def combine(b: String): String = a + b

  def combineAll[A](xs: List[A])(using Semigroup[A]): A = xs.reduce((a, b) => a.combine(b))

enum Shape:
  case Circle(r: Int)
  case Rect(w: Int, h: Int)
  case Empty

object Shapes:
  export Shape.*

  def area(s: Shape): Int = s match
    case Circle(r) => 3 * r * r
    case Rect(w, h) => w * h
    case Empty => 0

case class Money(cents: Int):
  def +(other: Money): Money = Money(cents + other.cents)

object Money:
  val zero: Money = Money(0)
  def of(units: Int): Money = Money(units * 100)

object Config:
  val retries: Int = 3
  var verbose: Boolean = false
  def label(n: Int): String = s"try $n of $retries"

  object Limits:
    val max: Int = 99
    def check(n: Int): Boolean = n <= max

trait Base[A]:
  def wrap(a: A): List[A] = List(a, a)
  extension (a: A) def twiceWith(f: A => A): A = f(f(a))

object IntOps extends Base[Int]:
  def wrapped: List[Int] = wrap(1)
  def grown: Int = 5.twiceWith(_ * 3)

// The exports of a class are visible in its body.
class Report(title: String):
  export Config.{label, Limits}

  def lines: List[String] = List(title, label(1), Limits.check(100).toString)

def makeReport(): Report =
  println("making a report")
  Report("made")

// Inherited exports are needed for the header of a nested class before `Sessions` is completed.
object Sessions extends SessionTypes:
  case class Session(user: User, tags: List[Tag]) extends Tracked

  def start(name: String): Session = Session(User(name), List(Tag("new")))

trait SessionTypes extends MoreSessionTypes:
  export Models.User

trait MoreSessionTypes:
  export Models.{Tag, Tracked}

object Models:
  case class User(name: String)
  case class Tag(label: String)
  trait Tracked:
    def track: String = "tracked"

object Api:
  export Instances.given
  export Shapes.{area, Circle, Rect}
  export Shape.Empty as NoShape
  export demo.Money
  export Config.{retries as _, *}
  export IntOps.*

@main def run(): Unit =
  // extension methods that a given provides
  println(1.combine(2))
  println("a".combine("b"))
  println(combineAll(List(1, 2, 3)))

  // enum cases exported by an object that gets exported in turn
  val shapes: List[Shape] = List(Circle(2), Rect(2, 5), NoShape)
  println(shapes.map(area))
  shapes.foreach:
    case Circle(r) => println(s"circle $r")
    case Rect(w, h) => println(s"rect $w x $h")
    case NoShape => println("nothing")

  // a case class with a companion
  val total: Money = Money.of(2) + Money(50) + Money.zero
  println(total)
  println(Api.Money.zero)

  // hiding, nested objects, a var read through the export
  println(label(2))
  println(Limits.max)
  println(Api.Limits.check(5))
  println(verbose)

  // members that an object inherits from a generic trait
  println(wrap(7))
  println(4.twiceWith(_ + 1))
  println(wrapped)
  println(grown)

  // exported names on an instance: the receiver is still evaluated
  val report = Report("report")
  println(report.lines)
  println(report.label(7))
  println(makeReport().Limits.max)

  val session = Sessions.start("ann")
  println(session)
  println(session.track)
  val user: Sessions.User = Sessions.User("bob")
  println(user)
