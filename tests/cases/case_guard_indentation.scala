def accept(condition: Boolean): Boolean = condition
object Guards:
  def choose(xs: List[Int]): List[Int] = xs.collect {
    case x if
      (x > 0) => x
  }
  def colon(x: Any): String = x match
    case s: String if accept:
      s.length > 4
    => s
    case _ => "short"
  def nested(xs: List[Int | Double]): List[Int] =
    xs.map:
      case i: Int =>
        7 match
        case j: Int
        if xs.exists: k =>
          xs.exists(_ == j && j == k)
        => 1
        case _ => 2
      case _ => 3
@main def indentedGuards(): Unit =
  println(Guards.choose(List(-1, 1)))
  println(Guards.colon("longer"))
  println(Guards.nested(List[Int | Double](7, 3.14)))
