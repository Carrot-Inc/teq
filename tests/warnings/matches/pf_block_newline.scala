// A block of a partial function's literal whose first statement shares the line of its `{`, the
// match on the lines below: the block is parsed as scalac parses it, and its match is the
// block's result (E211) or a block of its own (checked for exhaustivity) as on one line.
sealed trait Shape
case class Circle(r: Int) extends Shape
case class Square(s: Int) extends Shape

object Main:
  val nextLine: PartialFunction[Shape, Any] = s => { val k = 1
    s match
      case Circle(r) => r + k
  }
  val braced: PartialFunction[Shape, Any] = s => { val k = 1
    s match { case Circle(r) => r + k }
  }
  val nested: PartialFunction[Shape, Any] = s => { val k = 1
    { s match { case Circle(r) => r + k } }
  }
  def main(args: Array[String]): Unit =
    println(List(nextLine, braced, nested).map(_.isDefinedAt(Square(0))))
