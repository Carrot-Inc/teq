class R(val n: Int)
trait G {
  def n: Int
  given a(using Nothing): R = new R(n)
  given b: R = new R(n)
}
object P extends G { def n = 1 }
object Q extends G { def n = 2 }
object E { export P.a; export Q.b }

object Warm { import P.{a, b}; val warm = summon[R] }
object Test { import E.{a, b}; val result = summon[R].n }

@main def main(): Unit = println(Test.result)
