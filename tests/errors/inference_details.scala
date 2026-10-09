// Type variables constrained across parameter lists, folds over empty collections, members of a union.
// expect: 20:61: error: type mismatch: found (Int, Int), required (Nothing, V1)
// expect: 21:31: error: type mismatch: found Map[Nothing, Nothing], required Map[Int, Int]
// expect: 33:12: error: method nullary in object F must be called with () argument
trait Shape
case class Circle(r: Int) extends Shape
case class Square(s: Int) extends Shape
trait Greets { def hello: String }
case class GA() extends Greets { def hello = "ga" }
case class GB() extends Greets { def hello = "gb" }

object F:
  val c = true
  val i: Int = 3
  def flat[A](a: A)(b: A)(c: A): List[A] = List(a, b, c)
  val r13 = flat(1)(2L)(3)
  val r13t: List[Int | Long] = r13                 // ok? A collects bounds across all lists
  val fold = List(1, 2).foldLeft(Nil)((acc, x) => x :: acc)   // accepted, as scalac accepts it
  val foldt: List[Int] = fold
  val fold2 = List(1, 2).foldLeft(Map())((acc, x) => acc + (x -> x))
  val fold2t: Map[Int, Int] = fold2
  val v12 = if c then (Circle(1): Circle | Square) else Circle(3)
  val w12: Circle | Square = v12                   // ok? non-singleton hard union in a branch
  def hello(x: GA | GB): String = x.hello
  val w5: List[? <: AnyVal] = List("a")            // scalac: error?
  def taking(fn: Int => Int): Int = fn(1)
  val t3 = taking:
    x => x + 1
  def tuplesAny(x: Any): String = x match
    case (_, _) => "pair"
    case _ => "other"
  def nullary(): Int = 1
  val g2 = nullary
  val b4 = if c then 1 else 'a'
  val b2 = if c then i else 2L
  val b2kind = b2 match
    case x: Int => "int"
    case x: Long => "long"
  val l3 = List(i, 4L)
  val l3kind = l3.head match
    case x: Int => "int"
    case x: Long => "long"
  val bothLits = List(1, 2L)
  val bothLitsKind = bothLits.head match
    case x: Int => "int"
    case x: Long => "long"
  val mixed = List(1, "a")
  val mixedKind = mixed.head match
    case x: Int => "int"
    case _ => "other"

@main def main(): Unit =
  println(F.r13)
  println(F.fold)
  println(F.hello(GA()))
  println(F.tuplesAny((1, 2)))
  println(F.b4.isInstanceOf[Char])
  println(F.b2kind)
  println(F.l3kind)
  println(F.bothLitsKind)
  println(F.mixedKind)
