import scala.util.Try

sealed trait Shape
case class Circle(r: Int) extends Shape
case class Rect(w: Int, h: Int) extends Shape

var guards = 0
def big(n: Int): Boolean =
  guards += 1
  n > 2

def describe(pf: PartialFunction[Shape, String], s: Shape): String =
  if pf.isDefinedAt(s) then pf(s) else "undefined"

def firstDefined[A, B](xs: List[A])(pf: PartialFunction[A, B]): Option[B] =
  xs.find(pf.isDefinedAt).map(pf)

def bigCircle: PartialFunction[Shape, String] =
  case Circle(r) if big(r) => "big circle " + r

def endsWithFour: PartialFunction[String, Int] =
  case s if s.endsWith("4") => 4

@main def main(): Unit =
  val pf: PartialFunction[Int, String] =
    case 1 => "one"
    case n if n > 10 => "big " + n
  println(pf.isDefinedAt(1))
  println(pf.isDefinedAt(5))
  println(pf(1))
  println(pf.apply(11))
  println(Try(pf(2)).isFailure)
  println(pf.lift(5))
  println(pf.lift(12))
  println(pf.applyOrElse(7, (n: Int) => "other " + n))
  println(pf.applyOrElse(1, (n: Int) => "other " + n))

  val braces: PartialFunction[Int, String] = {
    case 0 => "zero"
    case n if n < 0 => "negative"
  }
  println(braces.isDefinedAt(-3))
  println(braces.isDefinedAt(3))

  val q = pf.orElse[Int, String] {
    case 5 => "five"
  }
  println(q(5))
  println(q(1))
  println(q.isDefinedAt(6))
  println(List(1, 5, 20).map(q))

  val lengths = pf.andThen(_.length)
  println(lengths.lift(1))
  println(lengths.lift(2))
  val chained = pf.andThen(endsWithFour)
  println(chained.isDefinedAt(14))
  println(chained.isDefinedAt(13))
  println(chained.lift(14))

  println(describe(bigCircle, Circle(3)))
  println(describe(bigCircle, Circle(1)))
  println(describe(bigCircle, Rect(1, 2)))
  println(guards)
  println(bigCircle.lift(Circle(5)))
  println(guards)

  val total: Shape => Int =
    case Circle(r) => r
    case Rect(w, h) => w * h
  println(total(Rect(2, 3)))

  val doubled: PartialFunction[Int, Int] = x => x * 2
  println(doubled.isDefinedAt(4))
  println(doubled(4))
  val viaMatch: PartialFunction[Int, Int] = x => x match
    case 1 => 100
  println(viaMatch.isDefinedAt(2))
  println(viaMatch.isDefinedAt(1))

  println(firstDefined(List(Rect(1, 1), Circle(9), Circle(1))) { case Circle(r) => r })
  println(firstDefined(List[Shape](Rect(1, 1))) { case Circle(r) => r })

  val ascribed = ({ case n: Int if n > 0 => "pos" }: PartialFunction[Int, String])
  println(ascribed.isDefinedAt(0))
  println(ascribed(2))

  println(PartialFunction.empty[Int, Int].isDefinedAt(1))
  println(PartialFunction.fromFunction((x: Int) => x + 1).lift(1))
  val unlifted = Function.unlift((x: Int) => if x > 0 then Some(x) else None)
  println(unlifted.isDefinedAt(-1))
  println(unlifted(3))
  val halves = ((x: Int) => Option.when(x % 2 == 0)(x / 2)).unlift
  println(halves.lift(4))
  println(halves.lift(3))
  println(PartialFunction.cond(3) { case n if n > 2 => true })
  println(PartialFunction.cond(1) { case n if n > 2 => true })
  println(PartialFunction.condOpt(3) { case 4 => "four" })
  println(PartialFunction.condOpt(4) { case 4 => "four" })

  val runner = bigCircle.runWith(s => println("ran " + s))
  println(runner(Circle(9)))
  println(runner(Circle(0)))
  val composed = bigCircle.compose((n: Int) => Circle(n))
  println(composed(7))
  val positiveCircle: PartialFunction[Int, Circle] = { case n if n > 0 => Circle(n) }
  val composedPartial = bigCircle.compose(positiveCircle)
  println(composedPartial.lift(7))
  println(composedPartial.lift(-7))
  println(composedPartial.lift(1))

  val pairs: PartialFunction[(Int, String), String] =
    case (1, s) => s
    case (n, s) if n > 5 => s + n
  println(pairs.lift((1, "a")))
  println(pairs.lift((3, "b")))
  println(pairs.lift((7, "c")))
  println(pairs.isDefinedAt((7, "c")))
