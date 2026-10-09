// Calls of values whose function type names its parameters, in the shapes other than a plain
// call: explicit `using` arguments, by position or by name, whose own type fixes the result
// (an anonymous class with `type T = Int`), named arguments written out of order and bound to
// temporaries, a value reached through a singleton type, a type parameter's bound or an
// intersection or a further refinement, a case lambda for two parameters, a refinement whose
// result is narrower than its parent's (which overloading sees), a plain function over a wider
// parameter where one naming it is expected, and results that apply an abstract type member or
// a type parameter, approximated at their variances where a plain function type is asked for.
trait C:
  type T
  def value: T
object I extends C:
  type T = Int
  def value = 7
object S extends C:
  type T = String
  def value = "s"
trait H:
  type F[A] <: List[A]
  def get: F[Int]
object L extends H:
  type F[A] = List[A]
  def get = List(1, 2)
trait K:
  type T <: String
  type F[+A] <: List[A]
  def get: F[T]
object KS extends K:
  type T = String
  type F[+A] = List[A]
  def get = List("k")
class Narrow extends (Any => Any):
  def apply(x: Any): String = "value"

object Main:
  var trace = List.empty[String]
  def next[A <: C](a: A, label: String): a.type =
    trace = label :: trace
    a
  def call[F <: (c: C) => c.T](f: F): Int = f(I)
  def choose(x: Any): String = "any"
  def choose(x: Int): String = "int"
  def choose(x: String): String = "string"
  def widen[F[+_]](f: (k: K) => F[k.T]): K => F[String] = f

  def main(args: Array[String]): Unit =
    val ctx: (c: C) ?=> c.T = (c: C) ?=> c.value
    val anon: Int = ctx(using new C { type T = Int; def value = 5 })
    val byName: String = ctx(using c = S)
    println(s"$anon $byName")
    val two: (a: C, b: C) => (a.T, b.T) = (a, b) => (a.value, b.value)
    val swapped: (Int, String) = two(b = next(S, "b"), a = next(I, "a"))
    println(s"$swapped ${trace.reverse}")
    val f: (c: C) => c.T = c => c.value
    val g: f.type = f
    val n: Int = g(I)
    val both: ((c: C) => c.T) & AnyRef = f
    val m: String = both(S)
    println(s"$n $m ${call(f)}")
    val cases: (c: C, k: Int) => List[c.T] = { case (_, _) => Nil }
    val none: List[Int] = cases(I, 0)
    println(none.size)
    val member: (h: H) => h.F[Int] = h => h.get
    val plain: H => List[Int] = member
    println(plain(L).sum)
    val narrow = new Narrow().asInstanceOf[(Any => Any) { def apply(x: Any): String }]
    val tagged = f.asInstanceOf[((c: C) => c.T) { type Tag = Int }]
    println(s"${choose(narrow(1))} ${choose(tagged(I))}")
    val wide: Any => Int = _ => 3
    val onString: (s: String) => Int = wide
    val kf: (k: K) => k.F[k.T] = k => k.get
    val kPlain: K => List[String] = kf
    val kList: K => List[String] = widen[List](k => List(k.get.head))
    println(s"${onString("s")} ${kPlain(KS)} ${kList(KS)}")
