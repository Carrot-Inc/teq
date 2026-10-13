// A type before a given's `=>` is a using clause of that one parameter (dotty's
// `Parsers.newTermParamssAndParents`: `given I[B] => J[B]` is a conditional given), a named
// one included; the conditional given of a trait is found through the objects extending it, the
// same member under two prefixes ranked by them (`pos/implicit-prefix-disambiguation`).
class I[X](val n: Int)
class J[X](val n: Int)
object O:
  given I[String] = I(1)
  given I[String] => J[String] = J(summon[I[String]].n + 1)
  given ji: (i: I[Int]) => J[Int] = J(i.n + 10)
  given I[Int] = I(5)
  given I[Long] => I[Char] => J[Long] = J(summon[I[Long]].n + summon[I[Char]].n)
  given I[Long] = I(100)
  given I[Char] = I(1000)
class K[X](val tag: String)
trait A:
  given K[B] = K("A")
  given K[B] => J[B] = J(summon[K[B]].tag.length)
object A extends A
trait B extends A
object B extends B
@main def run(): Unit =
  import O.given
  println(summon[J[String]].n)
  println(summon[J[Int]].n)
  println(summon[J[Long]].n)
  println(summon[K[B]].tag)
  println(summon[J[B]].n)
