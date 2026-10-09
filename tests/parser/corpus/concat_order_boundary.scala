//> using platform js
// Where a chain of `+` ends for scalac's JVM backend: behind an ascription, a cast, a `toString`
// of a string and any other call around its head. It goes on through parentheses, a block, an
// inline parameter and the branch a constant condition selects.
import scala.compiletime.summonFrom
import scala.language.implicitConversions

class Bad:
  override def toString: String =
    println("  render")
    throw new RuntimeException("toString")

class Loud(s: String):
  override def toString: String =
    println("  render " + s)
    s

def tail(): String =
  println("  tail")
  "!"

def cond(): Boolean = true

object Consts:
  final val T = true
  inline val I = true
  val V = true

type S = String

inline def pick(inline c: Boolean, x: Any): String = (if c then "" + x else "") + tail()

inline def picked(c: Boolean, x: Any): String = (if c then "" + x else "") + tail()

inline def iif(inline c: Boolean, x: Any): String = (inline if c then "" + x else "") + tail()

inline def imatch(inline n: Int, x: Any): String = (inline n match { case 1 => "" + x; case _ => "" }) + tail()

inline def cat(inline l: String, inline r: String): String = l + ":" + r

inline def wrapped(inline s: String): String = (s: String) + tail()

inline def selected(inline s: String): String = s.toString + tail()

class Holder:
  def locked(x: Any): String = synchronized { "" + x } + tail()

class Start

given Conversion[Start, String] with
  def apply(x: Start): String = "s:"

trait Log:
  def name: String

given Log with
  def name = "log"

inline def found(x: Any): String = summonFrom { case l: Log => l.name + x; case _ => "none" } + tail()

def attempt(body: => String): Unit =
  try println(body)
  catch case e: RuntimeException => println("  caught")

@main def main(): Unit =
  println("a throwing toString behind a boundary")
  println("toString"); attempt(("" + new Bad).toString + tail())
  println("ascription"); attempt((("" + new Bad): String) + tail())
  println("cast"); attempt(("" + new Bad).asInstanceOf[String] + tail())
  println("constant if"); attempt((if true then "" + new Bad else "") + tail())
  println("if"); attempt((if cond() then "" + new Bad else "") + tail())
  println("parentheses"); attempt(("" + new Bad) + tail())

  println("a builder behind a boundary")
  val sb = new java.lang.StringBuilder("a")
  def add(): String =
    sb.append("b")
    ""
  def again(s: String): String =
    sb.setLength(1)
    s
  println(again(("" + sb).toString + add()))
  println(again((("" + sb): String) + add()))
  println(again(("" + sb).asInstanceOf[String] + add()))
  println(again((if true then "" + sb else "") + add()))
  println(again((if cond() then "" + sb else "") + add()))
  println(again(("" + sb) + add()))

  val l = new Loud("x")
  println("erased wrappers")
  println("toString()"); println(("" + l).toString() + tail())
  println("alias ascription"); println((("" + l): S) + tail())
  println("annotated"); println((("" + l): @unchecked) + tail())
  println("cast to alias"); println(("" + l).asInstanceOf[S] + tail())
  println("synchronized"); println(new Holder().locked(l))
  println("two wrappers"); println((("" + l): String).toString + tail())
  println("toString in a block"); println(({ println("  stat"); ("" + l).toString }) + tail())
  println("ascribed block"); println((({ println("  stat"); "" + l }): String) + tail())
  println("cast of a block"); println(({ println("  stat"); "" + l }).asInstanceOf[String] + tail())
  println("interpolation ascribed"); println((s"$l!": String) + tail())
  println("interpolation toString"); println(s"$l!".toString + tail())
  println("chain after a boundary"); println((("" + l): String) + new Loud("y") + tail())

  println("calls")
  println("nn"); println(("" + l).nn + tail())
  println("locally"); println(locally("" + l) + tail())
  println("identity"); println(identity("" + l) + tail())
  println("concat"); println(("" + l).concat("c") + tail())
  println("f interpolator"); println(f"$l%s!" + tail())
  println("match"); println((("" + l) match { case s => s }) + tail())
  println("try"); println((try "" + l finally println("  finally")) + tail())

  println("what the chain goes through")
  println("double parentheses"); println((("" + l)) + tail())
  println("block without statements"); println(({ "" + l }) + tail())
  println("block in a block"); println(({ println("  outer"); { println("  inner"); "" + l } }) + tail())

  println("conditions")
  println("if false"); println((if false then "" else "" + l) + tail())
  println("if final val"); println((if Consts.T then "" + l else "") + tail())
  println("if inline val"); println((if Consts.I then "" + l else "") + tail())
  println("if val"); println((if Consts.V then "" + l else "") + tail())
  println("if folded"); println((if 1 < 2 then "" + l else "") + tail())
  println("if negated"); println((if !false then "" + l else "") + tail())
  println("nested constant ifs"); println((if true then (if false then "" else "" + l) else "") + tail())
  println("constant if around toString"); println((if true then ("" + l).toString else "") + tail())
  println("toString of a constant if"); println((if true then "" + l else "").toString + tail())
  println("block ending in a constant if"); println(({ println("  stat"); if true then "" + l else "" }) + tail())
  println("constant if with a block as its condition"); println((if { println("  cond"); true } then "" + l else "") + tail())
  println("constant if whose branch is an interpolation"); println((if true then s"$l!" else "") + tail())

  println("inline methods")
  println("if on an inline parameter"); println(pick(true, l))
  println("if on a bound parameter"); println(picked(true, l))
  println("inline if"); println(iif(true, l))
  println("inline match"); println(imatch(1, l))
  println("inline parameter ascribed"); println(cat(("" + l): String, tail()))
  println("inline parameter toString"); println(cat(("" + l).toString, tail()))
  println("inline parameter cast"); println(cat(("" + l).asInstanceOf[String], tail()))
  println("inline parameter constant if"); println(cat(if true then "" + l else "", tail()))
  println("inline parameter ascribed in the body"); println(wrapped("" + l))
  println("inline parameter selected in the body"); println(selected("" + l))

  println("a + written as a call, on a conversion to String"); println(new Start().+(l) + tail())
  println("summonFrom"); println(found(l))
