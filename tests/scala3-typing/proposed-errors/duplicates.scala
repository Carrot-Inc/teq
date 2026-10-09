// expect: z is already defined
// expect: type T is already defined
object O:
  type T = Int
  type T = String

def block(): Int =
  val z = 1
  val z = 2
  z

@main def run(): Unit = println(block())
