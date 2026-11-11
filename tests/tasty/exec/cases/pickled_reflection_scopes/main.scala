object Main:
  def increment(value: Int): Int = value + 1
  def main(args: Array[String]): Unit =
    println(ReflectionScopes.direct(increment(2)))
    println(ReflectionScopes.nested(increment(3)))
    println(ReflectionScopes.staged(increment(4)))
