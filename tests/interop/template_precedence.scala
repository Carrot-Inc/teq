// A template and a conditional written where JavaScript's precedence would take them apart get
// parentheses: a conditional, an assignment or an arrow as a conditional's condition, a template
// with an operator as an operand, a receiver or a callee, a prefix operator as a receiver. The
// expectations are Scala's meaning of each line; a comment gives what the master of 2026-09-28
// printed where it was wrong.
import scala.scalajs.js
import scala.scalajs.js.annotation.JSGlobalScope

class Box(val n: Int):
  def get: Int = n
  def twice(): Int = n * 2

object T:
  @js("$1 ? 'a' : 'bb'") def pick(v: Boolean): String
  @js("$1 + $2") def add(a: Int, b: Int): Int
  @js("$1 || $2") def or(a: Boolean, b: Boolean): Boolean
  @js("$1 === 1 ? $2 : $3") def choose(k: Int, a: Box, b: Box): Box
  @js("$1 ? (x) => x + 1 : (x) => x * 2") def fn(v: Boolean): js.Function1[Int, Int]
  @js("-$1") def neg(a: Int): Int
  @js("$parseInt($1) << 4 >> 2") def shifted(s: String): Int
  @js("globalThis.TP_FLAG = $1") def setFlag(b: Boolean): Unit

@js.native @JSGlobalScope
object Globals extends js.Object:
  val TP_FLAG: Boolean = js.native
  val TP_MISSING: Boolean = js.native

object Main:
  var count = 0
  def tick(b: Boolean): Boolean = { count += 1; b }
  def main(args: Array[String]): Unit =
    val v = args.length == 0
    val w = !v
    println(T.pick(v).length)  // master: a
    println(T.pick(w).length)
    println(T.pick(v) + "!")  // master: a
    println(T.pick(w) == "bb")
    println(if T.pick(v) == "a" then "yes" else "no")  // master: a
    println(if T.or(w, w) then "or" else "nor")
    println(T.add(1, 2) * 10)
    println(-T.add(1, 2))  // master: 1
    println(T.add(1, 2).toString.length)
    println(T.neg(5).toString)
    println(T.neg(5) + T.neg(-2))
    println(T.shifted("3") + 1)  // master: 6
    println(T.choose(1, new Box(3), new Box(4)).get)  // master: Box@1
    println(T.choose(2, new Box(3), new Box(4)).twice())
    println(T.fn(v)(10))
    println(T.fn(w)(10))
    println(if (if v then true else false) then "a" else "b")  // master: true
    println(if (if tick(w) then tick(true) else tick(false)) then "c" else "d")
    println(count)
    println((if (if v then T.pick(v) == "a" else false) then 1 else 2) + 10)  // master: 0
    println(if Globals.TP_MISSING then "set" else "unset")  // master: an empty line
    T.setFlag(false)
    println(if Globals.TP_FLAG then "flag" else "no flag")
    T.setFlag(true)
    println(if Globals.TP_FLAG then "flag" else "no flag")
    val f = (b: Boolean) => T.pick(b)
    println(f(v) + f(w))
    println(List(T.pick(v), T.pick(w)).map(_.length))
    println(s"${T.pick(v)}-${T.add(2, 3)}")  // master: a
    println(Array(T.add(1, 1), T.neg(1)).toList)
