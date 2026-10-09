// A method with parameters that is named without arguments becomes a function value,
// whatever type is expected.

def add(a: Int, b: Int): Int = a + b
def inc(x: Int): Int = x + 1
def greet(name: String, punctuation: String = "!"): String = "hello " + name + punctuation
def count(xs: Int*): Int = xs.length

def single[T](x: T): List[T] = List(x)
def pairUp[A, B](a: A, b: B): (A, B) = (a, b)
def firstOf[T](xs: List[T]): T = xs.head
def twice(x: => Int): Int = x + x

def applyAll(fs: List[Int => Int], x: Int): List[Int] = fs.map(f => f(x))

def describe(x: Any): String = x match
  case _: String => "a string"
  case _ => "not a string"

object Ops:
  def triple(x: Int): Int = x * 3

class Counter(step: Int):
  def next(x: Int): Int = x + step

@main def run(): Unit =
  val f = add
  println(f(1, 2))
  val g = inc
  println(g(g(1)))
  val fs = List(inc, Ops.triple, Counter(10).next)
  println(applyAll(fs, 5))
  println(describe(inc))
  val h = greet
  println(h("teq", "?"))
  val stored: Any = add
  println(describe(stored))
  def local(x: Int): Int = x - 1
  val l = local
  println(List(1, 2, 3).map(l).map(inc))
  val c: Seq[Int] => Int = count
  println(c(List(1, 2, 3)))
  println(List(List(1), List(2, 3)).map(count))
  val pair = (inc, "inc")
  println(pair._1(41) + " " + pair._2)
  // type parameters that only occur in parameter position become Any
  val s = single
  println(s(1) ++ s("one"))
  val p = pairUp
  println(p(1, "x"))
  val first = firstOf
  println(first(List("a", 2)))
  println(List(1, 2).map(single))
  // eta-expansion over a by-name parameter
  val t = twice
  println(t(4))
