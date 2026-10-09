package fix.reader

import scala.annotation.targetName
import fix.rdecl.*

// Library bodies whose references the reader keeps exactly as scalac 3.8.4 then resolved them,
// compiled against reader_decl_v1.scala and run against reader_decl.scala.
object RdCalls:
  // An overload of a class of another jar through an applied prefix, one alternative told
  // apart by its target name; a generic prefix; an object's overloads of one erasure.
  def applied(b: RdBox[String]): String = b.put("s") + ", " + b.put(1)
  def generic[T](b: RdBox[T], t: T): String = b.put(t)
  def targets: Int = RdNamed.sum(List(1, 2)) + RdNamed.sum(List("ab"))
  // Declared in `RdSquare` when this body was compiled, in its parent `RdShape` in the jar.
  def inherited(s: RdSquare): String = s.area(2)
  // Compiled when `RdTok.f`, `g` and `h` had one alternative each, run against two.
  def tokens: String = RdTok.f(new RdTokB.Token) + ", " + RdTok.g(Array("x")) + ", " + RdTok.h(RdWrap("s"))
  // A receiver of an intersection type, an array of a bounded wildcard and a Java class's
  // static nested class, each compiled when the name had the one alternative the jar keeps.
  def viaInter(x: RdMarker & RdApi): String = x.f("s")
  def boundedArray: String = RdArr.f(Array(1))
  def javaNested: String = fix.rjava.RdJava.f(new fix.rjava.RdSub)
  def localGeneric: String =
    def twice[A](a: A, f: A => A): A = f(f(a))
    twice[String]("x", _ + "y")
  // One capture against two: `b.get` is a `b.put(T)`'s argument, `d.get` only `putAny`'s.
  def sameCapture(a: Any): String = a match
    case b: RdBox[_] => b.put(b.get)
    case _ => "other"
  def distinctCaptures(a: Any, c: Any): String = (a, c) match
    case (b: RdBox[_], d: RdBox[_]) => b.put(d.get)
    case _ => "other"
  def bounded(a: Any): Int = a match
    case b: RdFooBox[_] => b.put(b.get)
    case _ => 0
  def nested(a: Any): String = a match
    case b: RdBox[t] =>
      b.get match
        case s: String => b.put(b.get) + " " + s
        case _ => "inner other"
    case _ => "outer other"
  def namedAndAnon(a: Any, c: Any): String = (a, c) match
    case (b: RdBox[t], d: RdBox[_]) =>
      val x: t = b.get
      b.put(x) + " " + b.put(d.get)
    case _ => "other"
  // Captures bounded as the pattern writes them, beyond what the class's parameter and the
  // scrutinee give: below, above, by an outer case's variable, a type parameter and a path.
  def lowerBounded(a: Any): String = a match
    case b: RdBox[? >: String] => b.put("ok")
    case _ => "other"
  def upperBounded(a: Any): Int = a match
    case b: RdBox[? <: String] => b.get.length
    case _ => -1
  def outerBound(a: Any, c: Any): String = a match
    case p: RdBox[t] => c match
      case q: RdBox[? <: t] =>
        val x: t = q.get
        "outer " + x
      case _ => "inner other"
    case _ => "other"
  def paramBound[A](a: Any, y: A): String = a match
    case b: RdBox[? >: A] => b.put(y)
    case _ => "other"
  def pathBound(h: RdStart)(a: Any): String = a match
    case b: RdBox[? >: h.Start] => b.put(h.start)
    case _ => "other"
  // A capture bounded below over a contravariant scrutinee, and by classes named `Any` and
  // `Nothing` that are not scala's.
  def contraLower(x: RdIn[Int]): String = x match
    case b: RdInBox[? >: String] => b.put("ok")
    case _ => "other"
  def customUpper(a: scala.Any): Int = a match
    case b: RdBox[? <: RdCustom.Any] => b.get.n
    case _ => -1
  def customLower(a: scala.Any): String = a match
    case b: RdBox[? >: RdCustom.Nothing] => b.put(new RdCustom.Nothing(3))
    case _ => "other"
  // Two transparent methods of one class, one expanding the other, and an argument inlined from
  // the caller's scope, whose `INLINED` has no call.
  def expansions(t: RdTraces): Int = t.one + t.two + new RdTraces().two
  def outerScope(t: RdTraces): Int = t.wrap(t.base(4))
  // Calls whose evidence the lean std's members do without.
  def arr(xs: List[Int]): Array[Int] = xs.toArray
  def unz(ps: List[(Int, String)]): (List[Int], List[String]) = ps.unzip
  // A parameter's wildcard captured as `TypeBox[Nothing, Any]#CAP`, and an `IArray` extension of
  // the jar whose body names `arr.T` with its class (`TYPEREFin`) under `--std=scala-library`.
  def capturedArray(x: Array[?]): String = x.headOption.fold("none")(_.toString)
  def firstOfIArray(xs: IArray[Int]): Int = xs.head

// An abstract type a pattern's bound names through a path.
trait RdStart:
  type Start
  def start: Start

// A member of the class reached through `C.this` by an inline call's argument, whose singleton
// type the expansion's result names (scala-library's `RedBlackTree.TreeIterator.stackOfNexts`).
final class RdCursor(n: Int):
  private val stack: Array[Int] | Null = if n > 0 then new Array[Int](n) else null
  def size: Int = if stack == null then 0 else stack.nn.length

// An object's overloads called from its own bodies, whose pickle names the alternative by its
// address (`TERMREFsymbol`): with one capture only the generic one applies.
object RdPick:
  def put[A](b: RdBox[A], x: A): String = "put[A] " + x
  @targetName("putAnyBox") def put(b: RdBox[Any], x: Any): String = "put(RdBox[Any]) " + x
  def same(a: Any): String = a match
    case b: RdBox[_] => put(b, b.get)
    case _ => "other"

// Overloads of the same file, reached by symbol from the class's own bodies.
final class RdLocal[T](val v: T):
  def put(x: T): String = "local put(T) " + x
  @targetName("putAny") def put(x: Any): String = "local put(Any) " + x
  def self: String = put(v)
  def other(o: RdLocal[?]): String = o match
    case l: RdLocal[_] => put(l.v)

// Type variables a pattern binds, one of them over a path (`RdFn[a, c.Start]`).
sealed trait RdEv[+A]
final case class RdNow[A](value: A) extends RdEv[A]
final case class RdMemo[A](ev: RdEv[A], var result: Option[A] = None) extends RdEv[A]
final case class RdFn[A, B](ev: RdEv[A], f: A => B) extends RdEv[B]

trait RdFM[A]:
  type Start
  def start(): RdEv[Start]
  def run: Start => RdEv[A]

object RdEvs:
  def evaluate[A](c: RdFM[A]): RdEv[A] = c.start() match
    case m: RdMemo[a] =>
      m.result match
        case Some(a) => c.run(a)
        case None => c.run(evalNow(m.ev))
    case RdNow(v) => c.run(v)
    case fn: RdFn[a, c.Start] =>
      val x: a = evalNow[a](fn.ev)
      c.run(fn.f(x))
  def evalNow[A](e: RdEv[A]): A = e match
    case RdNow(v) => v
    case m: RdMemo[a] => evalNow[a](m.ev)
    case fn: RdFn[a, A] => fn.f(evalNow[a](fn.ev))
  def program: RdFM[String] = new RdFM[String]:
    type Start = Int
    def start() = RdMemo(RdNow(4))
    def run = i => RdNow((i * 10).toString)
