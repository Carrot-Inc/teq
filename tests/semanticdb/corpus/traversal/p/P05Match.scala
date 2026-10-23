package p05
object Matches:
  def m(x: Any): Int = x match
    case i: Int if i > 0 => i
    case s @ (_: String) => s.length
    case Some(v) => 1
    case List(a, b @ _*) => b.size
    case l: List[t] => 0
    case 1 | 2 => 3
    case _ => -1
  def t(): Int =
    try 1
    catch
      case e: RuntimeException => e.hashCode
      case _: Throwable => 2
    finally println("x")
  val handler: PartialFunction[Throwable, Int] = { case _ => 0 }
  def t2 = try 1 catch handler
