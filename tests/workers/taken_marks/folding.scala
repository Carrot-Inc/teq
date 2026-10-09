// The conditions scalac folds away and the ones it keeps: ascriptions and casts, alone and inside
// a comparison, `null`, concatenations, nested conditions, blocks.
class Loud(s: String):
  override def toString: String =
    println("  render " + s)
    s

def tail(): String =
  println("  tail")
  "!"

type Bool = Boolean
type T = true
object K:
  final val F = true
  val v = true

@main def main(): Unit =
  val l = new Loud("x")
  println("ascribed to Boolean"); println((if (true: Boolean) then "" + l else "") + tail())
  println("ascribed to the singleton"); println((if (true: true) then "" + l else "") + tail())
  println("ascribed to an alias of Boolean"); println((if (true: Bool) then "" + l else "") + tail())
  println("ascribed to an alias of the singleton"); println((if (true: T) then "" + l else "") + tail())
  println("ascribed to Boolean, compared"); println((if (true: Boolean) == true then "" + l else "") + tail())
  println("ascribed to Boolean, negated"); println((if !(false: Boolean) then "" + l else "") + tail())
  println("ascribed to Boolean, conjunction"); println((if (true: Boolean) && true then "" + l else "") + tail())
  println("ascribed to Any, compared"); println((if (true: Any) == true then "" + l else "") + tail())
  println("ascribed Int, compared"); println((if (1: Int) == 1 then "" + l else "") + tail())
  println("ascribed Int, arithmetic"); println((if (1: Int) + 1 == 2 then "" + l else "") + tail())
  println("ascribed String, compared"); println((if ("a": String) == "a" then "" + l else "") + tail())
  println("final val ascribed to Boolean"); println((if (K.F: Boolean) then "" + l else "") + tail())
  println("final val ascribed, compared"); println((if (K.F: Boolean) == true then "" + l else "") + tail())
  println("cast to Boolean"); println((if true.asInstanceOf[Boolean] then "" + l else "") + tail())
  println("cast to Boolean, compared"); println((if true.asInstanceOf[Boolean] == true then "" + l else "") + tail())
  println("cast Int, compared"); println((if 1.asInstanceOf[Int] == 1 then "" + l else "") + tail())
  println("cast to the singleton"); println((if true.asInstanceOf[true] then "" + l else "") + tail())
  println("toString of a string, compared"); println((if "a".toString == "a" then "" + l else "") + tail())
  println("toString of an int, compared"); println((if 1.toString == "1" then "" + l else "") + tail())
  println("null == null"); println((if null == null then "" + l else "") + tail())
  println("null != null"); println((if null != null then "" else "" + l) + tail())
  println("concat of constants, compared"); println((if ("a" + "b") == "ab" then "" + l else "") + tail())
  println("concat with an int, compared"); println((if ("a" + 1) == "a1" then "" + l else "") + tail())
  println("concat with a boolean, compared"); println((if ("a" + true) == "atrue" then "" + l else "") + tail())
  println("block with a pure binding"); println((if { val unused = 3; true } then "" + l else "") + tail())
  println("block with a binding of a call"); println((if { val unused = tail(); true } then "" + l else "") + tail())
  println("block with a binding of a final val"); println((if { val u = K.F; u } then "" + l else "") + tail())
  println("block with a binding of a val"); println((if { val u = K.v; true } then "" + l else "") + tail())
  println("block with a lazy binding"); println((if { lazy val u = tail(); true } then "" + l else "") + tail())
  println("block with a def"); println((if { def u = tail(); true } then "" + l else "") + tail())
  println("string equality of constants"); println((if "ab" == "a" + "b" then "" + l else "") + tail())
  println("char comparison"); println((if 'a' < 'b' then "" + l else "") + tail())
  println("long comparison"); println((if 1L == 1L then "" + l else "") + tail())
  println("double comparison"); println((if 1.5 > 1.0 then "" + l else "") + tail())
  println("mixed comparison"); println((if 1 == 1L then "" + l else "") + tail())
  println("string length"); println((if "ab".length == 2 then "" + l else "") + tail())
