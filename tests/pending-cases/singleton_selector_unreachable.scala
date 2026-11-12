// teq: --source future
// A type test on a `Singleton` selector under `-source future`: scalac 3.8.4
// warns that the selector should be an instance of `Matchable` (`Singleton`'s
// one parent is `Any`, Definitions.scala:554-556) and rejects the case as
// unreachable (E030, an error). teq is to give the warning and reject the case too.
object E:
  def unapply(x: Int): Boolean = true
def f(x: Singleton): Boolean = x match
  case E() => true
  case _ => false
