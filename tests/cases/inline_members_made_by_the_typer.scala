// The calls of inline methods that the typer writes for another construct are expanded like calls written
// by hand: an indexed assignment operator over an inline `apply` (and an inline `update`), a `for` over inline
// `map`, `flatMap`, `withFilter` and `foreach`, a pattern over an inline `unapply`, an application of a
// value with an inline `apply`, an inline conversion, a default argument that is an inline call and an
// inline method's own default, an inline interpolator.
package meters

import scala.language.implicitConversions

class Row:
  private val cells = Array(1, 2, 3)
  inline def apply(i: Int): Int = cells(i)
  def update(i: Int, v: Int): Unit = cells(i) = v

class Strip:
  private val cells = Array(1, 2, 3)
  inline def apply(i: Int): Int = cells(i)
  inline def update(i: Int, v: Int): Unit = cells(i) = v

class Crate(val items: List[Int]):
  inline def map(inline f: Int => Int): Crate = Crate(items.map(f))
  inline def flatMap(inline f: Int => Crate): Crate = Crate(items.flatMap(x => f(x).items))
  inline def withFilter(inline p: Int => Boolean): Crate = Crate(items.filter(p))
  inline def foreach(inline f: Int => Unit): Unit = items.foreach(f)

object Half:
  inline def unapply(n: Int): Option[Int] = if n % 2 == 0 then Some(n / 2) else None

object Twice:
  inline def apply(n: Int): Int = n * 2

class Scale(k: Int):
  inline def apply(n: Int): Int = n * k

final class Wrapped(val n: Int)
inline given Conversion[Int, Wrapped] = n => Wrapped(n + 1)
inline implicit def fromText(s: String): Wrapped = Wrapped(s.length)
def unwrap(w: Wrapped): Int = w.n

object Sizes:
  inline def two: Int = 2
  def sized(xs: List[Int], upTo: Int = two): Int = xs.take(upTo).length
  inline def padded(n: Int, by: Int = two): Int = n + by

extension (sc: StringContext)
  inline def tagged(args: Any*): String = sc.parts.mkString("|") + args.length

@main def run(): Unit =
  val r = Row()
  r(1) += 10
  val s = Strip()
  s(1) += 10
  println(s"${r(1)} ${s(1)}")
  val made = for x <- Crate(List(1, 2, 3)) if x > 1; y <- Crate(List(10)) yield x + y
  for x <- made do print(s"$x ")
  println()
  val halved = 8 match
    case Half(h) => h
    case _ => -1
  val scale = Scale(3)
  println(s"$halved ${Twice(4)} ${scale(5)} ${unwrap(4)} ${unwrap("abc")}")
  val seven = 7
  println(s"${Sizes.sized(List(1, 2, 3))} ${Sizes.padded(5)} ${Sizes.padded(5, 1)} ${tagged"a${seven}b"}")
