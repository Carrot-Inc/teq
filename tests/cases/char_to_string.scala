// A Char's toString is a String that compares as one, on every target.
object Main:
  def main(args: Array[String]): Unit =
    val s = '('.toString
    println(s == "(")
    println(s.equals("("))
    println("(" == s)
    println(s.length)
    println(s match
      case "(" => "open"
      case _ => "other")
    val any: Any = s
    println(any == "(")
    println(any match
      case "(" => "open"
      case _ => "other")
    println(List('a', 'b').map(_.toString).mkString("-"))
    println('x'.toString + 'y')
    println(Set('q'.toString).contains("q"))
