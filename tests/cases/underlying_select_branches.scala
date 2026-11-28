class V(val i: Int) extends AnyVal

object Main:
  def operand(i: Int): Int =
    println("operand " + i)
    i

  def fromIf(first: Boolean): Int =
    (if first then new V(operand(11))
     else new V(operand(12)))
      .i

  def fromMatch(n: Int): Int =
    (n match
      case 0 => new V(operand(21))
      case _ => new V(operand(22)))
      .i

  def main(args: Array[String]): Unit =
    println(fromIf(true))
    println(fromIf(false))
    println(fromMatch(0))
    println(fromMatch(1))
