// A bare `given` selector ending its statement: before the end of an indented block (an object's body, a method's),
// a semicolon, or a newline; the selector is read as `given <type>` only when a type follows.
object P:
  given Int = 7
  def hi = "hi"
object O:
  val x = 1
  export P.given
object Q:
  export P.hi
object Main:
  def f: Unit =
    import P.given
  def g: Int =
    import P.given; summon[Int]
  def main(args: Array[String]): Unit =
    import O.given
    println(summon[Int])
    println(Q.hi)
    f
    println(g)
