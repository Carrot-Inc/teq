class TC(val n: Int)
object TC { given TC = new TC(0) }
trait G[A] { def a: A; given g: A = a }

object S extends G[String] { def a = "s"; val warm = summon[TC] }
object I extends G[TC] { def a = new TC(1); val result = summon[TC].n }

@main def main(): Unit = println(I.result)
