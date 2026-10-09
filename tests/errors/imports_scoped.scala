// expect: cannot resolve import: Missing not found
// `import Counts.count.*` below is accepted: a stable val is a valid qualifier (scalac agrees).
// expect: 15:11: error: not found: Red
// expect: 19:22: error: not found: Green
// expect: 22:23: error: not found: Blue
// expect: 31:18: error: not found: Blue

enum Color:
  case Red, Green, Blue

object Counts:
  val count: Int = 1

def early(): Unit =
  val a = Red
  import Color.*
  val b = Red

def outside: Color = Green

object Members:
  def before: Color = Blue
  import Color.*
  def after: Color = Blue

def inner(): Unit =
  val c =
    import Color.*
    Blue
  val d: Color.type = Color
  val e: Color = Blue

def unresolved(): Unit =
  import Missing.*
  import Counts.count.*
