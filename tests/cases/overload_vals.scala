//> using scala 3.8.4
trait Sized:
  def size: Int
  def size(unit: String): String = size.toString + unit
class Box(val size: Int) extends Sized
class Bag extends Sized:
  def size: Int = 3
  override def size(unit: String): String = "bag " + size + unit
def show(s: Sized): String = s.size.toString + "/" + s.size("cm")
@main def run(): Unit =
  println(show(Box(2)))
  println(show(Bag()))
  println(Box(5).size + Bag().size)
