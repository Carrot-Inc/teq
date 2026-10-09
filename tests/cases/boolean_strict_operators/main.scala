// `&`, `|` and `^` of a Boolean are ordinary methods that evaluate both operands, the left
// first; only `&&` and `||` skip their right operand.
import scala.annotation.tailrec

object Log:
  var count = 0
  def effect(tag: String, v: Boolean = true): Boolean =
    count += 1
    println("evaluated " + tag)
    v
  def once(tag: String): Boolean = effect(tag)

object K:
  final val T = true
  final val F = false
  final val Zero = 0

inline def orInline(a: Boolean, b: Boolean): Boolean = a | b
inline def andInline(inline a: Boolean, inline b: Boolean): Boolean = a & b
inline def xorInline(inline a: Boolean, inline b: Boolean): Boolean = a ^ b
inline def pick(): String = inline if true | false then "folded" else "kept"

def attempt(tag: String)(b: => Boolean): Unit =
  println(tag + ": " + (try b.toString catch { case e: Exception => e.getClass.getSimpleName }))

// The right operand of `|` is no tail position: the recursion goes down to -1.
def rec(n: Int): Boolean = if n < 0 then false else (n == 0 | rec(n - 1))

@tailrec def recShort(n: Int): Boolean = n <= 0 || recShort(n - 1)

@main def run(): Unit =
  import Log.effect
  var s = "  x "
  var t = true
  var f = false
  var zero = 0
  attempt("throws or")((1 != 2) | (s.toInt > 0))
  attempt("throws and")((1 == 2) & (s.toInt > 0))
  attempt("throws xor")(true ^ (1 / zero > 0))
  attempt("short or")((1 != 2) || (s.toInt > 0))
  attempt("short and")((1 == 2) && (s.toInt > 0))
  println(t | effect("or"))
  println(!t & effect("and"))
  println(t ^ effect("xor"))
  println(t || effect("never"))
  println(f && effect("never"))
  println(effect("left", false) & effect("right"))
  println(effect("left", true) | effect("right", false))
  println(effect("l", true) ^ effect("m", true) ^ effect("r", false))
  // A constant operand folds nothing away.
  println(true | effect("true or"))
  println(false & effect("false and"))
  println(K.T | effect("K.T or"))
  println(K.F & effect("K.F and"))
  attempt("constant division")(false & (1 / K.Zero > 0))
  attempt("folded division")((K.F && (1 == 2)) & ((1000000007 / (0: Byte)).toChar == 3L))
  println(pick())
  // The values are Booleans, not the numbers of JavaScript's `&` and `|`.
  val stored = t & f
  println(stored)
  println("r=" + (t | f) + " " + (t & t) + " " + (f ^ t))
  println(List(t & f, t | f, t ^ f))
  println(s"${(t | f).hashCode} ${(t & f).hashCode}")
  println((t | f) == true)
  // Boxes and functions.
  val bx: java.lang.Boolean = true
  println(bx | effect("box or"))
  println(bx.booleanValue & effect("box and"))
  val any: Any = false
  println(any.asInstanceOf[Boolean] & effect("any and"))
  val ob: Option[Boolean] = Some(true)
  println(ob.get | effect("option or"))
  println(ob.exists(_ | effect("lambda or")))
  val g: (Boolean, Boolean) => Boolean = _ | _
  println(g(true, effect("function argument")))
  println(List(true, false, true).reduce(_ & _))
  println(List(effect("a"), effect("b", false)).foldLeft(false)(_ | _))
  // Assignment operators.
  var v = true
  v |= effect("or-assign")
  v &= effect("and-assign", false)
  v ^= effect("xor-assign")
  println(v)
  // Conditions.
  if t | effect("if or") then println("if or taken") else println("if or not")
  if f & effect("if and") then println("if and taken") else println("if and not")
  if !(f & effect("if not and")) then println("not taken") else println("not not")
  if !(t | effect("if not or")) then println("not taken") else println("not not")
  if t ^ effect("if xor") then println("xor taken") else println("xor not")
  var i = 0
  while (i < 2) & effect("while " + i) do i += 1
  println("i=" + i)
  println(if (t | effect("nested")) && (f | effect("nested2", false)) then "both" else "not both")
  val chosen = if f & effect("chosen") then 1 else 2
  println(chosen)
  // Inline expansions.
  println(orInline(true, effect("inline or")))
  println(andInline(false, effect("inline and")))
  println(xorInline(K.T, effect("inline xor")))
  println(Show.either(true, effect("macro")))
  println(Show.show(t & Log.once("x")))
  println(Show.show(t | Log.once("x")))
  println(Show.show(t && Log.once("x")))
  println(Show.show(t ^ Log.once("x")))
  println(rec(4))
  println(recShort(3))
  println("effects " + Log.count)
