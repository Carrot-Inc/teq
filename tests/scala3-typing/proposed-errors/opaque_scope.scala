// expect: Found:    String⏎Required: A
// expect: Found:    A⏎Required: String
opaque type A = String
val ok: A = "abc"

object obj:
  val y: A = "abc"
  def f(a: A): String = a

@main def run(): Unit = println(obj.y)
