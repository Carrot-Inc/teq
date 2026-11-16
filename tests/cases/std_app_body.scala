// `scala.App`: the body of an object that extends `App` is the object's initialiser, run when the program
// starts and before the inherited `main`'s own code: its statements in order with another object's, its vals, a
// lazy val, a var and a def, and a trait between the object and `App`. `args` is null there: `main` has not run.
object Ledger { val opened: Int = { println("ledger opened"); 3 } }
trait Stall extends App { println("stall set up") }
object Test extends Stall {
  println("start")
  val goods: Int = Ledger.opened + 4
  lazy val tally: Int = { println("tallying"); goods * 2 }
  var sold: Int = 0
  def sell(n: Int): Int = { sold += n; sold }
  println(goods)
  println(sell(2) + sell(3))
  println(tally)
  println(args == null)
  println(executionStart > 0L)
  println(this.isInstanceOf[DelayedInit])
  println("end")
}
