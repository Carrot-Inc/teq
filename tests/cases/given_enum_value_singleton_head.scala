// A given of an enum's class is no candidate for its value's singleton at an invariant position
// (`TypeComparer.isSubArgs`), whichever way the search reads the target's head: the value's own
// given is taken, and the enum's for the enum.
trait Show[A]:
  def show(a: A): String
enum Color:
  case Red, Blue
object Use:
  given Show[Color] with
    def show(a: Color) = "color " + a
  given Show[Color.Red.type] with
    def show(a: Color.Red.type) = "red " + a
  given [T <: Color]: Show[List[T]] with
    def show(as: List[T]) = as.mkString("list ", ",", "")
@main def run(): Unit =
  import Use.given
  println(summon[Show[Color.Red.type]].show(Color.Red))
  println(summon[Show[Color]].show(Color.Blue))
  println(summon[Show[List[Color.Red.type]]].show(List(Color.Red)))
