// The trigger of `int-to-double-text` in proptests/src/known.rs, shrunk by tests/prop-known.sh:
// 1 of the program's lines differ, of 1 cases
// line 0.f.t: on jvm the folded form prints "0" and the other "0.0"
// folded: (0: Short).toDouble
// at run time: Vars.kv0.toDouble
//   where var kv0: Short = 0
// the failure's kind: t: folded differs on jvm
//> using platform js
object Consts:
  final val none = 0


object Vars:
  var none: Int = 0
  var k0v0: Short = 0

def units(text: String): String =
  var out = ""
  var i = 0
  while i < text.length do
    out = out + text.charAt(i).toInt + " "
    i += 1
  out

def show(id: String, text: => String): Unit =
  val shown =
    try text
    catch
      case e: ArithmeticException => "ArithmeticException"
      case e: IndexOutOfBoundsException => "IndexOutOfBoundsException"
      case e: NumberFormatException => "NumberFormatException"
  println(id + ":" + shown)

def r0v(): String =
  java.lang.Double.doubleToLongBits((Vars.k0v0.toDouble)).toString

def r0t(): String =
  (Vars.k0v0.toDouble).toString

@main def run(): Unit =
  show("0.f.v", java.lang.Double.doubleToLongBits(((0: Short).toDouble)).toString)
  show("0.f.t", ((0: Short).toDouble).toString)
  show("0.r.v", r0v())
  show("0.r.t", r0t())
