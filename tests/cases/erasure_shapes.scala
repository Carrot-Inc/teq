// The shapes whose erasure follows dotty's `TypeErasure` on the JVM (tests/support/abi/erasure) run alike: a `*:`
// chain as a bound (`Product`) and behind an alias (scalac's `Product`, teq's tuple class, left with E1), a union
// of `String` and a class (their erased lub), an applied opaque type over an array (`[I` for `Arr[Int]`, `Object`
// for a generic one).
type C2 = Int *: String *: EmptyTuple
object Arrays:
  opaque type Arr[A] = Array[A]
  object Arr:
    def ints(n: Int): Arr[Int] = Array.tabulate(n)(i => i * 2)
    def strs(xs: String*): Arr[String] = xs.toArray
  extension [A](a: Arr[A])
    def at(i: Int): A = a(i)
    def size: Int = a.length
  def sum(a: Arr[Int]): Int = a.sum
  def first[A](a: Arr[A]): A = a(0)
import Arrays.*
class API:
  def alias(x: C2): C2 = x
  def first(x: C2): Int = x._1
  def bound[T <: Int *: EmptyTuple](x: T): T = x
  def make: C2 = (1, "m")
  def union(x: String | java.lang.StringBuilder): String | java.lang.StringBuilder = x
  def serial(x: String | List[Int]): String | List[Int] = x
@main def run() =
  val api = new API
  println(api.alias((1, "x")))
  println(api.first((2, "y")))
  println(api.bound(Tuple1(3)))
  println(api.make)
  println(api.union("u"))
  println(api.union(new java.lang.StringBuilder("sb")))
  println(api.serial(List(1, 2)))
  val a = Arr.ints(4)
  println(sum(a))
  println(a.at(3))
  println(first(a))
  println(first(Arr.strs("x", "y")))
  println(a.size)
