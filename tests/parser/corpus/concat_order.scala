//> using platform js
// A chain of `+` on the JVM evaluates every operand and renders them afterwards; under Scala.js
// each operand is rendered as its `+` is evaluated. The interpreter does the first, the
// JavaScript output the second (tests/jvm-expected has the JVM's answer).
class Loud(s: String):
  override def toString: String =
    println("  render " + s)
    s

class Thrower:
  override def toString: String =
    println("  render thrower")
    throw new RuntimeException("toString")

class Meters(val v: Int) extends AnyVal:
  override def toString: String =
    println("  render meters")
    v.toString + "m"

object Holder:
  println("  init Holder")
  override def toString: String =
    println("  render Holder")
    "Holder"

object Named:
  println("  init Named")
  val name = "name"

class Cell:
  var f = "f0"
  override def toString: String =
    f = "f1"
    "cell"

def effect(s: String): String =
  println("  effect " + s)
  s

def fail(s: String): String = throw new RuntimeException(s)

def byName(l: Loud, tail: => String): String = "" + l + tail

def plain(a: Any): String = "plain:" + a

def attempt(body: => String): Unit =
  try println(body)
  catch case e: RuntimeException => println("  threw " + e.getMessage)

@main def main(): Unit =
  val l = new Loud("x")
  val any: Any = l

  println("builder")
  val sb = new java.lang.StringBuilder("a")
  def append(s: String): String =
    sb.append(s)
    ""
  println("" + sb + append("b"))

  println("chain")
  println("1:" + l + effect("y") + new Loud("z"))

  println("var")
  var n = 1
  def bump(): String =
    n = 5
    ""
  println("" + n + bump() + n)
  var o: Any = new Loud("before")
  def swap(): String =
    o = new Loud("after")
    ""
  println("" + o + swap() + o)

  println("throwing toString after an effect")
  attempt("" + new Thrower + effect("later"))

  println("throwing operand before a rendering")
  attempt("" + l + fail("operand"))

  println("parentheses")
  println(("p:" + l) + effect("e"))
  println("a" + ("b" + l) + effect("c"))
  println("a" + l + ("b" + new Loud("inner") + effect("in")) + effect("out"))

  println("val")
  val head = "h:" + l
  println(head + effect("t"))

  println("+=")
  var s = "s:"
  s += l
  s += effect("e")
  println(s)
  s = s + l + effect("f")
  println(s)

  println("kinds")
  val c = 'c'
  val i = 1
  val d = 1.5
  val f = 2.5f
  val g = 2L
  val b: Byte = 3
  val h: Short = 4
  val nothing: String = null
  val nobody: Any = null
  println("" + c + i + d + f + g + b + h + true + nothing + nobody + null + new Meters(3) + effect("end"))
  println("" + new Meters(4) + c + effect("c"))

  println("explicit and implicit")
  println("" + any.toString + effect("explicit"))
  println("" + any + effect("implicit"))
  println("" + l.toString + effect("explicit, typed"))
  println("" + l + effect("implicit, typed"))
  val t: Any = new Thrower
  attempt("" + t.toString + effect("explicit"))
  attempt("" + t + effect("implicit"))
  println("" + i.toString + d.toString + g.toString + c.toString + effect("numbers"))

  println("lazy val")
  lazy val tail = effect("lazy")
  println("" + l + tail)

  println("by-name")
  println(byName(l, effect("by-name")))

  println("var read")
  var v = "v0"
  class Setter:
    override def toString: String =
      v = "v1"
      "setter"
  println("" + new Setter + v)

  println("field read")
  val cell = new Cell
  println("" + cell + cell.f)

  println("object")
  println("" + l + Holder)
  println("" + l + Named.name)

  println("heads")
  println(plain(l) + effect("plain"))
  println((if n == 5 then "t:" + l else "f") + effect("if"))
  println(({ val p = "b:"; p + l }) + effect("block"))

  println("interpolation")
  println(s"$l!" + effect("outside"))
  println(s"$l-$any" + effect("two"))
  println("" + new Loud("a") + s"${new Loud("b")}!" + effect("out"))
  println(s"${new Loud("i")}" + l + effect("after"))
