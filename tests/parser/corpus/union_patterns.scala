// A class pattern over a union takes its field types from every alternative the class can be
// an instance of, so type tests on the fields are checked at run time.

final class Tw(val value: String)
final case class Box[A](v: A)

type ClassArg = String | (String, Boolean) | Tw | (Tw, Boolean) | (Int, Int, Int)
type Opt = Option[Int] | Option[String]

def describe(x: ClassArg): String = x match
  case s: String => "string " + s
  case t: Tw => "tw " + t.value
  case (s: String, on) => "string pair " + s + " " + on.toString
  case (t: Tw, on) => "tw pair " + t.value + " " + on.toString
  case (a, b, c) => "triple " + (a + b + c).toString

def opts(o: Opt): String = o match
  case Some(i: Int) => "int " + (i + 1).toString
  case Some(s: String) => "string " + s
  case None => "none"

def boxes(b: Box[Int] | Box[String] | Int): String = b match
  case Box(i: Int) => "int box " + (i + 1).toString
  case Box(s: String) => "string box " + s.length.toString
  case n: Int => "plain " + n.toString

def eithers(e: Either[String, Int] | Either[Int, String]): String = e match
  case Left(s: String) => "left string " + s
  case Left(i: Int) => "left int " + i.toString
  case Right(s: String) => "right string " + s
  case Right(i: Int) => "right int " + i.toString

def nested(x: (Int, (String, Boolean)) | (Int, (Tw, Boolean))): String = x match
  case (n, (s: String, on)) => "string " + n.toString + s + on.toString
  case (n, (t: Tw, on)) => "tw " + n.toString + t.value + on.toString

def swapped(p: (Int, String) | (String, Int)): String = p match
  case (i: Int, s: String) => "int first " + i.toString + s
  case (s: String, i: Int) => "string first " + s + i.toString

@main def main(): Unit =
  println(describe("a"))
  println(describe(Tw("t")))
  println(describe(("a", true)))
  println(describe((Tw("t"), false)))
  println(describe((1, 2, 3)))
  val os: Option[String] = Some("x")
  val oi: Option[Int] = Some(1)
  println(opts(os))
  println(opts(oi))
  println(opts(None))
  val bs: Box[String] = Box("abc")
  val bi: Box[Int] = Box(1)
  println(boxes(bs))
  println(boxes(bi))
  println(boxes(5))
  val e1: Either[String, Int] = Left("a")
  val e2: Either[Int, String] = Left(1)
  val e3: Either[String, Int] = Right(2)
  val e4: Either[Int, String] = Right("b")
  println(eithers(e1))
  println(eithers(e2))
  println(eithers(e3))
  println(eithers(e4))
  val n1: (Int, (String, Boolean)) = (1, ("s", true))
  val n2: (Int, (Tw, Boolean)) = (2, (Tw("t"), false))
  println(nested(n1))
  println(nested(n2))
  println(swapped((1, "a")))
  println(swapped(("a", 1)))
  val mixed: List[ClassArg] = List("a", ("b", true), Tw("c"), (Tw("d"), false))
  println(mixed.map(describe).mkString(", "))
