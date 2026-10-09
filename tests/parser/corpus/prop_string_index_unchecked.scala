// The trigger of `string-index-unchecked` in proptests/src/known.rs, shrunk by tests/prop-known.sh:
// 2 of the program's lines differ, of 1 cases
// line 0.f.v: the targets print
//     interpreter: IndexOutOfBoundsException
//     javascript: 
//     scalac: IndexOutOfBoundsException
// folded: "".substring(2, 0)
// at run time: p0.substring(p1, p2)
//   where p0: String is ""
//   where p1: Int is 2
//   where p2: Int is 0
// line 0.r.v: the targets print
//     interpreter: IndexOutOfBoundsException
object Consts:
  final val none = 0


object Vars:
  var none: Int = 0


def units(text: String): String =
  var out = ""
  var i = 0
  while i < text.length do
    out = out + text.charAt(i).toInt + " "
    i += 1
  out

def span(s: String, a: Int, b: Int): Boolean = true

def within(s: String, i: Int): Boolean = true

def show(id: String, text: => String): Unit =
  val shown =
    try text
    catch
      case e: ArithmeticException => "ArithmeticException"
      case e: IndexOutOfBoundsException => "IndexOutOfBoundsException"
      case e: NumberFormatException => "NumberFormatException"
  println(id + ":" + shown)

def r0v(p0: String, p1: Int, p2: Int): String =
  units((p0.substring(p1, p2)))

@main def run(): Unit =
  show("0.f.v", units(("".substring(2, 0))))
  show("0.r.v", r0v("", 2, 0))
