// An inline member's forwarder is inline, its body the member's call on the path, which a call
// expands with the call's own arguments (dotty's `RetainedExportTermFlags` and
// `PrepareInlineable.registerInlineInfo` of the forwarder): an inline parameter stays a constant,
// a transparent result stays precise, through a top-level value and through a class's value.
import scala.compiletime.error

class Lib:
  transparent inline def f(inline x: Int): Int = inline if x == 1 then 7 else error("bad")
  inline def twice(inline x: Int): Int = x + x
  transparent inline def pick(inline b: Boolean): Any = inline if b then "text" else 42

val lib = new Lib
export lib.{f, pick}

class Proxy(val lib: Lib):
  export lib.{f, twice}

class Box[A](val a: A):
  inline def get: A = a
  inline def map[B](inline f: A => B): B = f(a)
class BoxProxy[A](val box: Box[A]):
  export box.{get, map}

@main def run(): Unit =
  println(f(1))
  val s: String = pick(true)
  println(s.length)
  val p = new Proxy(new Lib)
  println(p.f(1) + p.twice(5))
  val n: Int = pick(false)
  println(n + 1)
  val b = new BoxProxy(new Box("x"))
  val got: String = b.get
  println(got + b.map(_.length))
