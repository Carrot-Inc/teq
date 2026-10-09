class R(val n: Int)
object R { given R = new R(0) }
object G { given g(using String): R = new R(1) }

object Warm {
  import G.g
  given String = ""
  val x = summon[R]
}
object Cold {
  import G.g
  val x = summon[R]
}
object Test {
  given R = new R(2)
  def result: Int = {
    import G.g
    summon[R].n
  }
}
@main def main(): Unit = println(Test.result)
