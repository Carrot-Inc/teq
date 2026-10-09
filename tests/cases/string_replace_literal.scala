// `String.replace` of a string or a char replaces the target as it is written and inserts the
// replacement as it is written: `$&`, `$1` and `$$` are text, as in Java, where JavaScript's
// `replaceAll` of a string reads them as patterns.
object Main:
  def main(args: Array[String]): Unit =
    println("a.b".replace(".", "$&$&"))
    println("aXb".replace("X", "$1"))
    println("aXb".replace("X", "$$"))
    println("aXbX".replace('X', 'Y'))
    println("aXbX".replace("X", "$`"))
    println("ab".replace("", "$'"))
    println("a.b.c".replace(".", "\\"))
    val dollar = "$&"
    println("xyx".replace("x", dollar) + "|" + "xyx".replace('x', '$'))
