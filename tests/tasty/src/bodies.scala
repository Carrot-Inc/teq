package fix.bodies

import scala.compiletime.{summonInline, erasedValue, constValue}

trait Pretty[A]:
  def show(a: A): String

object Pretty:
  given Pretty[Int] with
    def show(a: Int): String = a.toString
  given [A](using s: Pretty[A]): Pretty[List[A]] with
    def show(as: List[A]): String = as.map(s.show).mkString("[", ", ", "]")

case class Point(x: Int, y: Int)

class Base:
  def greet(name: String): String = "hello " + name

class Bodies extends Base:
  def block(n: Int): Int =
    val doubled = n * 2
    var acc = 0
    acc += doubled
    acc

  def lambda(xs: List[Int]): List[Int] = xs.map(x => x + 1)

  def matching(v: Any): String = v match
    case n: Int if n > 0 => "positive"
    case 0 | -1 => "zero or minus one"
    case Point(x, y) if x == y => s"diagonal $x"
    case p @ Point(_, _) => p.toString
    case s: String => s
    case _ => "other"

  def trying(f: () => Int): Int =
    try f()
    catch
      case e: IllegalArgumentException => -1
      case _: RuntimeException => -2
    finally println("done")

  def looping(n: Int): Int =
    var i = 0
    var sum = 0
    while i < n do
      sum += i
      i += 1
    sum

  def localDef(n: Int): Int =
    def twice(k: Int): Int = k * 2
    class Counter(start: Int):
      def next: Int = start + 1
    twice(new Counter(n).next)

  def byName(cond: Boolean, x: => Int): Int = if cond then x else 0

  def usesByName: Int = byName(true, block(3))

  def varargs(xs: Int*): Int = xs.sum

  def callsVarargs: Int = varargs(1, 2, 3) + varargs(List(4, 5)*)

  inline def describe[T]: String = inline erasedValue[T] match
    case _: Int => "int"
    case _: String => "string"
    case _ => "other"

  inline def showInline[A](a: A): String = summonInline[Pretty[A]].show(a)

  inline def choose(inline flag: Boolean): Int = inline if flag then 1 else 2

  transparent inline def pick(inline flag: Boolean): Any = inline if flag then block(1) else "one"

  def usesInline: String = describe[Int] + showInline(42) + choose(true)

  def usesTransparent: Int = pick(true)

  def usesGiven(xs: List[Int])(using s: Pretty[List[Int]]): String = s.show(xs)

  def interpolated(name: String, n: Int): String = s"$name has ${n + 1} items"

  def early(xs: List[Int]): Int =
    var rest = xs
    while rest.nonEmpty do
      if rest.head > 10 then return rest.head
      rest = rest.tail
    -1

  def earlyFromLambda(xs: List[Int]): Int =
    for x <- xs do if x > 10 then return x
    -1

  def failing(msg: String): Nothing = throw new IllegalStateException(msg)

  def nothingHere: String = null

  def construct: List[Int] = new ::[Int](1, Nil)

  override def greet(name: String): String = super.greet(name).toUpperCase

  class Inner(val label: String):
    def describeOuter: String = label + block(1)
    class Innermost:
      def reach: Int = block(2)
      def reachInlined: Int = pick(true)
