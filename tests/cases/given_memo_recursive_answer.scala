import scala.util.NotGiven

trait A
trait Root extends A
class R(val n: Int)

object R { given fallback: R = new R(0) }
object Root { given root(using r: => R): Root with {} }

given absent(using NotGiven[A]): R = new R(1)
val warm = summon[Root]

@main def main(): Unit = println(summon[R].n)
