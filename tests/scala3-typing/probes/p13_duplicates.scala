object O:
  val x = 1
  val x = 2
  def y = 1
  val y = 2
  type T = Int
  type T = String
  class K
  object K
  trait K
def block(): Int =
  val z = 1
  val z = 2
  z
@main def run(): Unit = println(block())
