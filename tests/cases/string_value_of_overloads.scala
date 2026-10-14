// `java.lang.String.valueOf`'s overloads, each by its own parameter: a `Char`, an `Int`, a `Boolean`, an
// object, the characters of an array.
object Main:
  def main(args: Array[String]): Unit =
    println(java.lang.String.valueOf('a'))
    println(java.lang.String.valueOf('b') + java.lang.String.valueOf(1) + java.lang.String.valueOf(true))
    println(java.lang.String.valueOf(Array('x', 'y', 'z')))
    println(java.lang.String.valueOf(List(1): Any))
