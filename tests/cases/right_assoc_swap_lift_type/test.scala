// The temporary a right-associative call binds its impure left operand to has the operand's widened type.
class C
class R:
  def %:(x: C): String = "ok"
val c: C = new C
def left(): c.type = c
def right(): R = new R
@main def run(): Unit = println(inspect(left() %: right()))
