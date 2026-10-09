// expect: 8:8: error: type T is already defined
// expect: 10:9: error: type K is already defined
// expect: 14:7: error: z is already defined
// expect: 16:7: error: f is already defined
// expect: 4 errors found
object O:
  type T = Int
  type T = String
  class K
  trait K

def block(): Int =
  val z = 1
  val z = 2
  def f = 1
  val f = 2
  z

@main def run(): Unit = println(block())
