// The temporary a right-associative call binds its impure left operand to is of a widened type, as a macro sees it.
class C
class R:
  def %:(x: C): String = "ok"
val c: C = new C
def left(): c.type = c
def right(): R = new R
@main def run(): Unit = println(inspect(left() %: right()))
