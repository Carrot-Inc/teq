// An enum whose cases lack a comma, and a case class with a broken parameter.
enum Color:
  case Red Green, Blue
case class P(x: Int, y: )
object O:
  val c: Color = Color.Green
  val p = P(1)
  val bad: String = 3
