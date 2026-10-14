// A body per construct of the interpreter's lowered code (src/interp/code.rs), each called often
// enough to be lowered, and the boundaries with the walker: a lowered body calling a body run once
// (which the walker runs) and back, nodes the walker takes inside lowered code (a concatenation, a
// default argument), a loop of a body run once that the lowered code takes over, a `return` out of
// a closure, a tail call through a lowered body and the exits of a `try`.
import scala.annotation.tailrec

sealed trait Shape
case class Circle(r: Int) extends Shape
case class Rect(w: Int, h: Int) extends Shape
case object Dot extends Shape

object Even:
  def unapply(n: Int): Option[Int] = if n % 2 == 0 then Some(n / 2) else None

class Counter(start: Int):
  var count: Int = start
  def this() = this(10)
  def bump(by: Int = 1): Int =
    count += by
    count

abstract class Animal:
  def sound: String
  def speak(times: Int): String = (1 to times).map(_ => sound).mkString("-")
class Dog extends Animal:
  def sound = "woof"
class Cat extends Animal:
  def sound = "meow"

object Registry:
  val names: List[String] = List("a", "b")
  var hits: Int = 0

def topLevel(x: Int): Int = x * 3

object Main:
  def literals(i: Int): String =
    val b: Byte = 7
    val s: Short = 300
    val c: Char = 'q'
    val l: Long = 1L << 40
    val f: Float = 1.5f
    val wide: Long = i
    val any: Any = b
    s"$b $s $c $l ${f > 1} $wide ${any.isInstanceOf[Byte]} ${i.toByte} ${'a' + i}"

  def locals(n: Int): Int =
    var acc = 0
    var k = 0
    while k < n do
      val sq = k * k
      acc += sq
      k += 1
    acc

  def calls(n: Int): String =
    val animals: List[Animal] = List(Dog(), Cat(), Dog())
    val c = new Counter()
    c.bump()
    c.bump(n)
    val f = (x: Int) => x + n
    def g(y: Int): Int = f(y) * 2
    s"${animals.map(_.speak(n)).mkString(",")} ${c.count} ${g(3)} ${topLevel(n)} ${Registry.names.size}"

  def shapes(s: Shape): String = s match
    case Circle(r) if r > 10 => s"big circle $r"
    case Circle(r) => s"circle $r"
    case Rect(w, h) if w == h => s"square $w"
    case Rect(1, _) | Circle(1) => "unit-ish"
    case Rect(w, _) => s"rect $w"
    case Dot => "dot"

  def numbers(n: Int): String = n match
    case 0 => "zero"
    case Even(half) => s"even $half"
    case x: Int if x < 0 => "negative"
    case _ => "odd"

  def seqs(xs: Seq[Int]): String = xs match
    case Seq() => "empty"
    case Seq(a) => s"one $a"
    case Seq(a, b, rest*) => s"$a $b +${rest.size}"

  def logic(n: Int): String =
    var trace = List.empty[Int]
    def t(i: Int, v: Boolean): Boolean = { trace = i :: trace; v }
    val r1 = t(1, n > 1) && t(2, n > 2)
    val r2 = t(3, n > 1) || t(4, n > 2)
    val r3 = !(n == 2) ^ (n == 3)
    s"$r1 $r2 $r3 ${trace.reverse.mkString}"

  def strings(n: Int): String =
    val parts = Array("x", "y", "z")
    val joined = "<" + parts(n % 3) + n + ">"
    s"${joined.length} ${joined.toUpperCase} ${joined.charAt(1)} ${n.toString * 2}"

  def casts(v: Any): String =
    try
      val s = v.asInstanceOf[String]
      s"string ${s.length}"
    catch case e: ClassCastException => "not a string"

  def arrays(n: Int): Int =
    val a = new Array[Int](n)
    var i = 0
    while i < n do
      a(i) = i * 2
      i += 1
    a.sum + varargs(1, 2, n)

  def varargs(xs: Int*): Int = xs.length * 100

  @annotation.nowarn
  def found(xs: List[Int], target: Int): Int =
    xs.foreach { x => if x == target then return x * 10 }
    -1

  def early(n: Int): String =
    var i = 0
    while true do
      if i == n then return s"stopped at $i"
      i += 1
    "never"

  @tailrec def sumTo(n: Int, acc: Int): Int = if n == 0 then acc else sumTo(n - 1, acc + n)

  @tailrec def closures(n: Int, fs: List[() => Int]): List[() => Int] =
    if n == 0 then fs else closures(n - 1, (() => n) :: fs)

  @tailrec def retry(n: Int): Int =
    try
      if n > 0 then throw new IllegalStateException("again") else 42
    catch case _: IllegalStateException => retry(n - 1)

  def exits(mode: Int): String =
    val log = new StringBuilder
    def body(): Int =
      try
        if mode == 0 then return 1
        if mode == 1 then throw new RuntimeException("boom")
        2
      catch
        case e: RuntimeException =>
          log.append("caught ")
          3
      finally log.append("finally ")
    val r = body()
    log.toString + r

  def walked(n: Int, label: String = "def"): String =
    // A concatenation and a default argument, which the walker evaluates inside lowered code.
    "walked " + label + " " + n + " " + once(n)

  def once(n: Int): Int =
    // Run once from a lowered body: the walker runs it, and its loop turns lowered after a while.
    var total = 0
    var i = 0
    while i < 100 do
      total += i % (n + 1)
      i += 1
    total + back(n)

  def back(n: Int): Int = n * 7

  @annotation.nowarn
  def folds(n: Int): String =
    // Case lambdas over the tuple of two parameters: literal components, guards, no case taken.
    val sum = (1 to n + 2).foldLeft(0) {
      case (acc, 2) => acc + 100
      case (acc, x) if x > n => acc + x * 10
      case (acc, x) => acc + x
    }
    val g: (Int, String) => String = {
      case (1, s) => "one " + s
      case (k, "x") => s"x $k"
    }
    val miss = try g(7, "y") catch case e: MatchError => "no match"
    s"$sum ${g(1, "a")} ${g(n, "x")} $miss"

  def main(args: Array[String]): Unit =
    for i <- 1 to 3 do
      println(literals(i))
      println(locals(i * 3))
      println(calls(i))
      println(List(Circle(i * 6), Rect(i, i), Rect(i, 2), Dot).map(shapes).mkString(", "))
      println(List(0, i, -i, i * 2).map(numbers).mkString(", "))
      println(List(Seq(), Seq(i), Seq(i, i, i, i)).map(seqs).mkString(", "))
      println(logic(i))
      println(strings(i))
      println(casts(if i == 2 then 42 else "s" * i))
      println(arrays(i))
      println(found(List(1, 2, 3), i) + " " + found(List(1, 2, 3), 9))
      println(early(i))
      println(sumTo(i * 1000, 0))
      println(closures(3, Nil).map(_()).mkString(","))
      println(retry(i))
      println(exits(i - 1))
      println(walked(i) + " " + walked(i, "given"))
      println(folds(i))
      Registry.hits += 1
    println(Registry.hits)
