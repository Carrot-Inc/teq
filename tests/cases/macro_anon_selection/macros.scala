import scala.quoted.*

// A member selected on a transparent macro's anonymous result outside the class's body: the
// pickle selects it through the parent the result's type names, as scalac's avoidance does.
object Lib:
  transparent inline def runner(inline n: Int): Runnable = ${ runnerImpl('n) }
  def runnerImpl(n: Expr[Int])(using Quotes): Expr[Runnable] =
    '{ new Runnable { def run(): Unit = println($n + 1) } }
