object Irrefutable {
  class R {
    def isEmpty: false = { println("isEmpty"); false }
    def get: Int = { println("get"); 7 }
  }
  object E { def unapply(n: Int): R = { println("E.unapply"); new R } }
  object T { def unapply(n: Int): true = { println("T.unapply"); true } }
  def main(args: Array[String]): Unit = {
    val E(a) = 1
    println(a)
    for E(b) <- List(1) do println(b)
    println(1 match { case T() => 9 })
  }
}
