// scala-library's reflective `Selectable`, as a pickled body calls it: a member reached by its
// name on the object itself, a method called, a field read, one with arguments applied.
import scala.reflect.Selectable.reflectiveSelectable

class Box(val size: Int):
  def label: String = "box of " + size
  def scaled(k: Int): Int = size * k

@main def run =
  val b: Any = new Box(3)
  println(reflectiveSelectable(b).selectDynamic("label"))
  println(reflectiveSelectable(b).selectDynamic("size"))
  println(reflectiveSelectable(b).applyDynamic("scaled", classOf[Int])(4))
