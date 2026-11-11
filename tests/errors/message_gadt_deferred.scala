enum Expr[T]:
  case IntLit(i: Int) extends Expr[Int]
def take(s: String): Int = 0
object Util:
  def take(s: String): Int = 0
def direct[T](e: Expr[T], t: T): String = e match
  case Expr.IntLit(i) => t
def applied[T](e: Expr[T], t: T): Int = take(e match
  case Expr.IntLit(i) => t)
def member[T](e: Expr[T], t: T): Int = Util.take(e match
  case Expr.IntLit(i) => t)
// A TypeMismatch reported inside a case reads the case's GADT bounds when it renders (Message.Seen's
// needsExplanation and explanation, Message.scala:198-201 and 220, over the creation Context's `ctx.gadt`).
// `direct` renders in the case; the argument of `take` and of `Util.take` defers its message past the
// application, after the case's GADT scope is gone, so the message keeps the constraint it was made under.
