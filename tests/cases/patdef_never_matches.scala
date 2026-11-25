// A pattern definition whose right side can never match its pattern (`@unchecked`: an Int against a tuple's or a
// case class's pattern, a case class's field of Int against a nested case class's) throws the MatchError of the
// value after the right side ran, in a method's body, a try's handler, a lambda, a class's field, a lazy val; and a
// match's case over an Int that a class pattern can never match is passed over. Nothing after the failing test
// reads the pattern's variables.
case class P(a: Int, b: Int)
case class Q(p: Int, c: Int)

class K {
  val (a, b) = { println("field rhs"); 1 }: @unchecked
}

object Main {
  def caught(what: String)(body: => Unit): Unit =
    try body catch { case e: MatchError => println(what + ": " + e.getMessage) }

  def tuple(): Unit = {
    val (a, b) = { println("tuple rhs"); 1 }: @unchecked
    println(s"$a $b")
  }

  def caseClass(): Unit = {
    val P(a, b) = (1: Any): @unchecked
    println(s"$a $b")
  }

  def nested(): Unit = {
    val Q(P(a, b), c) = { println("nested rhs"); Q(1, 2) }: @unchecked
    println(s"$a $b $c")
  }

  def inHandler(): Unit =
    try throw new RuntimeException("x")
    catch {
      case e: RuntimeException =>
        val (a, b) = { println("handler rhs"); 2 }: @unchecked
        println(s"$a $b")
    }

  def inLambda(): Unit = {
    val f = (x: Int) => { val (a, b) = { println("lambda rhs"); x }: @unchecked; a }
    println(f(3))
  }

  def lazyOne(): Unit = {
    lazy val (a, b) = { println("lazy rhs"); 4 }: @unchecked
    println("before")
    println(a)
  }

  def matched(): Int = ({ println("match rhs"); 5 }: @unchecked) match {
    case P(a, b) => a + b
    case _ => 0
  }

  def main(args: Array[String]): Unit = {
    caught("tuple")(tuple())
    caught("case class")(caseClass())
    caught("nested")(nested())
    caught("handler")(inHandler())
    caught("lambda")(inLambda())
    caught("field")(new K())
    caught("lazy")(lazyOne())
    println(matched())
  }
}
