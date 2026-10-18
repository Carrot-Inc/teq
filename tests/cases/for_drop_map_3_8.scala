// teq: --source 3.8 --wconf any:s
// `DropForMap` of a comprehension's trailing `map` (the review of 1888ac4df): the receiver where its type widened
// `=:=` the call's (`map: Any` keeps the call), else a conversion's argument where its type is the call's (a val's
// `TermRef` is not: the call stays); an extension's receiver its first argument; a leading or trailing using clause;
// the unit body; an inline `map`'s expansion replaced by the reference to its receiver's binding (dotty's
// `better-fors-map-inlined`, an explicit `this` over a call), a path receiver's expansion kept; and the tupling
// migration's application, warned of, typed as before.
import scala.language.implicitConversions
class AnyBox(val v: Int):
  def map(f: Int => Int): Any = { println("any map"); AnyBox(f(v)) }
class Raw(val v: Int)
class CBox(val v: Int):
  def map(f: Int => Int): Raw = { println("conv map"); Raw(f(v)) }
given Conversion[Raw, CBox] = r => CBox(r.v)
class Raw2(val v: Int)
class DBox(val v: Int):
  def map(f: Int => Int): Raw2 = { println("implicit def map"); Raw2(f(v)) }
implicit def toDBox(r: Raw2): DBox = DBox(r.v)
class EBox(val v: Int)
extension (b: EBox) def map(f: Int => Int): EBox = { println("ext map"); EBox(f(b.v)) }
class Ctx
given Ctx = Ctx()
class UBox(val v: Int):
  def map(using c: Ctx)(f: Int => Int): UBox = { println("using map"); UBox(f(v)) }
class TBox(val v: Int):
  def map(f: Int => Int)(using c: Ctx): TBox = { println("trailing using map"); TBox(f(v)) }
class Units(val xs: List[Unit]):
  def map(f: Unit => Unit): Units = { println("unit map"); Units(xs.map(f)) }
class IBox(val v: Int):
  inline def map(f: Int => Int): IBox = { println("inline map"); IBox(f(v)) }
class A2(val v: Int):
  inline def map(f: Int => Int): A2 = { println("inline this.v map"); A2(f(this.v)) }
def mk2(v: Int): A2 = A2(v)
class B1(val v: Int)
extension (b: B1) inline def map(f: Int => Int): B1 = { println("inline ext map"); b }
def mkB1(v: Int): B1 = B1(v)
enum MyOption[+A]:
  case MySome(x: A)
  case MyNone
  inline def map[B](f: A => B): MyOption[B] =
    this match
    case MySome(x) => { println("my map"); MySome(f(x)) }
    case MyNone => { println("my map none"); MyNone }
  def flatMap[B](f: A => MyOption[B]): MyOption[B] =
    this match
    case MySome(x) => f(x)
    case MyNone => MyNone
object MyOption:
  def apply[A](x: A): MyOption[A] = MySome(x)
def f(x: Int): (Int, Int) = (1, x)
@main def Test =
  println((for x <- AnyBox(1) yield x).isInstanceOf[AnyBox])
  val raw = Raw(1)
  println((for x <- raw yield x) eq raw)
  println((for x <- Raw(1) yield x).v)
  val raw2 = Raw2(2)
  println((for x <- raw2 yield x) eq raw2)
  println((for x <- Raw2(2) yield x).v)
  val e = EBox(3)
  println((for x <- e yield x) eq e)
  val u = UBox(6)
  println((for x <- u yield x) eq u)
  val t = TBox(7)
  println((for x <- t yield x) eq t)
  val us = Units(List((), ()))
  println((for x <- us yield ()) eq us)
  val i = IBox(4)
  println((for x <- i yield x) eq i)
  println((for x <- mk2(5) yield x).v)
  println((for x <- mkB1(6) yield x).v)
  println(for a <- MyOption(1); b <- MyOption(()) yield ())
  println(for a <- MyOption(1); b <- MyOption(2) yield b)
  println(for a <- MyOption(1); (b, c) <- MyOption((2, 3)) yield (b, c))
  val result: Iterable[(Int, Int)] =
    for
      (k, v) <- Map(1 -> 1, 2 -> 1, 3 -> 1)
      x      = k + v
      (a, b) = f(x)
      (y, z) <- Map(42 -> 27)
    yield (a+y, b+z)
  println(result)
  val l =
    for
      k <- List(1, 2)
      x = k + 1
      y = x + 1
      z <- List(10)
    yield y + z
  println(l)
