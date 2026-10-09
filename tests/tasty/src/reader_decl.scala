package fix.rdecl

import scala.annotation.targetName

// The declarations the bodies of reader.scala reach in another jar (`fixtures-rdecl` in
// tests/support/jars.sh): overloads told apart by `@targetName` on an applied prefix and on an
// object, a bounded class parameter, transparent inline methods of one class, and a method
// whose override was removed after the bodies were compiled (reader_decl_v1.scala has it) beside
// an overload of one term parameter that stays.
class RdBox[T](val value: T):
  def put(x: T): String = "put(T) " + x
  @targetName("putAny") def put(x: Any): String = "put(Any) " + x
  def get: T = value
  override def toString: String = "RdBox(" + value + ")"

trait RdFoo:
  def bar: Int

final class RdF1(n: Int) extends RdFoo:
  def bar: Int = n
  override def toString: String = "RdF1(" + n + ")"

final class RdFooBox[D <: RdFoo](val d: D):
  def get: D = d
  def put(x: D): Int = x.bar + get.bar
  def put(x: Any): Int = -1

object RdNamed:
  @targetName("sumInts") def sum(xs: List[Int]): Int = xs.sum
  @targetName("sumStrings") def sum(xs: List[String]): Int = xs.map(_.length).sum * 100

class RdTraces:
  def base(n: Int): Int = n * 10
  transparent inline def one: Int = base(1)
  transparent inline def two: Int = one + base(2)
  transparent inline def wrap(inline x: Int): Int = x + base(3)

class RdShape:
  def area(scale: Int): String = "shape " + scale

class RdSquare extends RdShape:
  def area(unit: String): String = "square in " + unit

// Overloads a signature tells apart by what the erasure keeps: a class's qualified name
// (`RdTokA.Token` and `RdTokB.Token`), a generic array's bound (`Array[A <: AnyRef]` is
// `Object[]`, `A` is `Object`) and a value class's argument (`RdWrap[String]` is `String`,
// where the class alone would be `Object`).
object RdTokA:
  class Token
object RdTokB:
  class Token extends RdTokA.Token
final class RdWrap[T](val v: T) extends AnyVal
object RdTok:
  def f(t: RdTokA.Token): String = "f(RdTokA.Token)"
  def f(t: RdTokB.Token): String = "f(RdTokB.Token)"
  def g[A](x: A): String = "g(A)"
  def g[A <: AnyRef](xs: Array[A]): String = "g(Array[A])"
  def h(x: Any): String = "h(Any)"
  def h(w: RdWrap[String]): String = "h(RdWrap[String])"

// A method of a class the receiver meets as one part of an intersection (`RdMarker & RdApi`).
trait RdMarker
class RdApi:
  def f(x: Any): String = "f(Any)"
  def f(x: String): String = "f(String)"

// A pattern's capture bounded below by `String` over a contravariant scrutinee (`RdIn[Int]`),
// whose narrowing gives it `Int` as well.
trait RdIn[-A]
class RdInBox[A](val a: A) extends RdIn[A]:
  def put(x: A): String = "put " + x

// An array of a wildcard bounded by `AnyVal`, whose erasure is `Object` (a generic array).
object RdArr:
  def f(xs: Array[? <: AnyVal]): String = "f(Array[? <: AnyVal])"
  def f(xs: Array[Int]): String = "f(Array[Int])"

// Classes named `Any` and `Nothing` of an object's own, which bound a pattern's captures.
object RdCustom:
  class Any(val n: Int)
  class Nothing(val m: Int) extends Any(m):
    override def toString: String = "Nothing(" + m + ")"
