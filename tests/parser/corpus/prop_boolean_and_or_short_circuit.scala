// The trigger of `boolean-and-or-short-circuit` in proptests/src/known.rs, shrunk by tests/prop-known.sh:
// 2 of the program's lines differ, of 1 cases
// line 0.f.v: the targets print
//     interpreter: true
//     javascript: true
//     jvm: true
//     scalac: IndexOutOfBoundsException
// folded: ((!((true & Consts.kc0))) | Consts.kc1.substring(Consts.kc2, 0).startsWith(Consts.kc3))
// at run time: ((!((Vars.kv0 & p0))) | Vars.kv1.substring(p1, Vars.kv2).startsWith(p2))
//   where inline val kc0 = false
//   where final val kc1 = "a\ud834\udd1eb"
//   where inline val kc2 = 2
//   where inline val kc3 = "\u00df"
object Consts:
  final val none = 0
  val written: 1 = 1
  inline val k0c0 = false
  final val k0c1 = "a\ud834\udd1eb"
  inline val k0c2 = 2
  inline val k0c3 = "\u00df"

object Vars:
  var none: Int = 0
  var k0v0: Boolean = true
  var k0v1: String = "a\ud834\udd1eb"
  var k0v2: Int = 0

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

def r0v(p0: Boolean, p1: Int, p2: String): String =
  (((!((Vars.k0v0 & p0))) | Vars.k0v1.substring(p1, Vars.k0v2).startsWith(p2))).toString

@main def run(): Unit =
  show("0.f.v", (((!((true & Consts.k0c0))) | Consts.k0c1.substring(Consts.k0c2, 0).startsWith(Consts.k0c3))).toString)
  show("0.r.v", r0v(false, 2, "\u00df"))
