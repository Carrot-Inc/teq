// Tuples of more than 22 elements written as tuples: a literal, a tuple type and a tuple pattern
// past 22 elements are scalac's `scala.runtime.TupleXXL`, read by index (`t(n)`, `head`, `tail`),
// so that one made by a literal and one made at run time (`Tuple.fromArray`, a mirror's
// `fromProductTyped`) are one class with one equality and hash; on the JVM such a tuple type is
// `scala.runtime.TupleXXL` in a descriptor, as scalac erases it.
import scala.compiletime.{constValueTuple, summonAll}
import scala.deriving.Mirror
import scala.reflect.ClassTag

def cls(x: Any): String = x.getClass.getName

object Api:
  def sum(t: (Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int)): Int = t(0) + t(11) + t(22)
  def same(t: (Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int)): (Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int) = t
  val held: (Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int) = (100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117, 118, 119, 120, 121, 122)
  def shape(x: Any): String = x match
    case (a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19, a20, a21, a22, a23) => "23 elements: " + a1 + " .. " + a23
    case (a, b) => "a pair " + a + " " + b
    case _ => "other"

object Split:
  def unapply(x: Int): Option[(Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int)] = Some((0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22))

def kind(x: Any): String = x match
  case _: (Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int) @unchecked => "a tuple type past 22 is any non-empty tuple to the test"
  case _ => "other"

case class Wide(
    a: Int, b: Int, c: Int, d: Int, e: Int, f: Int, g: Int, h: Int, i: Int, j: Int, k: Int, l: Int,
    m: Int, n: String, o: Int, p: Int, q: Int, r: Int, s: Int, t: Int, u: Int, v: Int, w: Int, x: Boolean)

@main def run(): Unit =
  val t = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23)
  println(cls(t) + " " + t)
  println(t(0).toString + " " + t(22) + " " + t.size + " " + t.head + " " + t.last)
  println(cls(t.tail) + " " + t.tail)
  println(cls(t.init) + " " + t.init.size)
  val c = 0 *: (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22)
  println(cls(c) + " " + c(22) + " " + c.size)
  val cc = -1 *: c
  println(cls(cc) + " " + cc.size + " " + cc.head + " " + cc(23))
  val joined = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12) ++ (50, 51, 52, 53, 54, 55, 56, 57, 58, 59, 60)
  println(cls(joined) + " " + joined.size + " " + joined(12))
  println(cls(t.zip(t)) + " " + t.zip(t)(22))
  println(t.toList.sum)
  println(Api.sum(t))
  println(Api.same(t) eq t)
  println(Api.held(22))
  println(Api.shape(t))
  println(Api.shape((1, 2)))
  println(Api.shape(Tuple.fromArray(Array.tabulate[Any](23)(i => i * 3))))
  val (a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19, a20, a21, a22, a23) = t
  println(a1 + a12 + a23)
  t match
    case h *: rest => println("cons pattern " + h + " " + cls(rest) + " " + rest.size)
  // One class whichever way the tuple was made: equality and hash as scalac's.
  val made = Tuple.fromArray(Array.tabulate[Any](23)(i => i + 1))
  println(t == made)
  println(made == t)
  println(t.hashCode == made.hashCode)
  println(t.hashCode)
  println((1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 33, 34).hashCode)
  println(t == (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23))
  println(t == Api.held)
  // A mirror's tuple of a 24-field case class is the same class, read by index.
  val wide = Wide(1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, "n", 15, 16, 17, 18, 19, 20, 21, 22, 23, true)
  val fields = Tuple.fromProductTyped(wide)
  println(cls(fields) + " " + fields.size + " " + fields(13) + " " + fields(23) + " " + fields.head)
  println(summon[Mirror.ProductOf[Wide]].fromProduct(fields) == wide)
  // The members of a product and the class, a tag, an array of them.
  println(t.productPrefix + " " + t.productArity + " [" + t.productElementName(0) + "] " + t.productIterator.toList.last)
  println(classOf[(Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int)].getName + " " + summon[ClassTag[(Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int)]].runtimeClass.getName)
  println(Array(t, t).map(_(22)).toList)
  // An extractor's result, a type test, a named tuple, untupled parameters, compile-time tuples.
  5 match
    case Split((a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19, a20, a21, a22, a23)) => println("extracted " + a1 + " " + a23)
  println(kind(t) + " / " + kind((1, 2)) + " / " + kind("s"))
  println((t: Any) match { case _: (Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int) @unchecked => true; case _ => false })
  val named = (f1 = 1, f2 = 2, f3 = 3, f4 = 4, f5 = 5, f6 = 6, f7 = 7, f8 = 8, f9 = 9, f10 = 10, f11 = 11, f12 = 12, f13 = 13, f14 = 14, f15 = 15, f16 = 16, f17 = 17, f18 = 18, f19 = 19, f20 = 20, f21 = 21, f22 = 22, f23 = 23)
  println(named.f1 + named.f23)
  named match
    case (f2 = b, f22 = v) => println("named pattern " + b + " " + v)
  println(List(t, t).map((a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19, a20, a21, a22, a23) => a2 + a22))
  for (a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19, a20, a21, a22, a23) <- List(t) do println(a4 + a20)
  val ls = constValueTuple[("l1", "l2", "l3", "l4", "l5", "l6", "l7", "l8", "l9", "l10", "l11", "l12", "l13", "l14", "l15", "l16", "l17", "l18", "l19", "l20", "l21", "l22", "l23")]
  println(cls(ls) + " " + ls(22))
  given Int = 7
  println(summonAll[(Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int)].toList.sum)
  // The members whose result scalac reduces on a tuple type: typed as the elements they keep.
  val small = (1, "a", 2.5)
  val kl: Double = small.last
  val ki: (Int, String) = small.init
  val kr: (Double, String, Int) = small.reverse
  val ka: (Int, String, Double, Char) = small :* 'c'
  val kt: (Int, String) = small.take(2)
  val kd: Tuple1[Double] = small.drop(2)
  val (kx, ky) = small.splitAt(1)
  println(kl.toString + ki + kr + ka + kt + kd + kx + ky + small.take(5) + small.drop(9) + small.take(0))
  val kl23: Int = t.last
  println(kl23.toString + " " + cls(t.init) + " " + t.reverse.head + " " + (t :* 24).size + " " + cls(t :* 24))
  println(cls(t.take(22)) + " " + cls(t.drop(1)) + " " + t.drop(1).head + " " + t.splitAt(22)._2)
  val k22 = (1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22) :* 23
  println(cls(k22) + " " + k22(22))
  val km: Option[Int] = t.map([a] => (x: a) => Option(x))(22)
  val kc: Int = t.init(21) + t.reverse(0) + t.take(23)(22) + t.drop(1)(21) + t.tail(0)
  println(km.toString + " " + kc + " " + (t :* "end")(23))
  // Named arguments of the builtins, and a tuple of one element, which is its own reverse.
  println((1, "a").:*(x = true).toString + " " + (1, "a").take(n = 1) + " " + (1, "a").splitAt(n = 1))
  val one = Tuple1(1)
  val dynamic: Tuple = Tuple1(2)
  println((one.reverse eq one).toString + " " + (dynamic.reverse eq dynamic))
