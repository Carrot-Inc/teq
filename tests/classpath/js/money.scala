// jars: scala-library cats-kernel cats-core
// A program over cats' Monoid, Show, Eq, Validated, Semigroupal and Option and Either syntax,
// compiled from the jars' TASTy bodies; the members it reaches cover the typer gaps it was
// written for.
//> using dep org.typelevel::cats-core:2.13.0
import cats.syntax.all._
import cats.{Eq, Eval, Monoid, Show}
import cats.data.Validated

case class Money(cents: Long)
object Money:
  given Monoid[Money] with
    def empty: Money = Money(0)
    def combine(a: Money, b: Money): Money = Money(a.cents + b.cents)
  given Show[Money] with
    def show(m: Money): String = f"$$${m.cents / 100}%d.${m.cents % 100}%02d"
  given Eq[Money] = Eq.fromUniversalEquals

def validAge(n: Int): Validated[List[String], Int] =
  if n >= 0 then Validated.valid(n) else Validated.invalid(List(s"negative: $n"))

@main def run(): Unit =
  val total = List(Money(150), Money(275), Money(5)).combineAll
  println(total.show)
  println((Money(1) |+| Money(2)) === Money(3))
  println(List(1, 2, 3).foldMap(_ * 2))
  println(Map("a" -> 1) |+| Map("a" -> 2, "b" -> 3))
  println((validAge(3), validAge(-1), validAge(-2)).mapN(_ + _ + _))
  println(List(Some(1), None, Some(3)).flatten.combineAll)
  println("x".some.orElse(None).show)
  println(Either.cond(true, 42, "no").toOption.show)
  println(Either.cond(true, 42, "no").leftMap(_.length))
  println(Eval.now(1).map(_ + 1).value)
  println(Eval.defer(Eval.later(21)).map(_ * 2).value)
