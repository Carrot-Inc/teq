object OldShapes {
  case class P(a: Int, b: Int)
  object E { def unapply(n: Int): Option[(Int, Int)] = { println("unapply " + n); Some((n, n + 1)) } }
  def main(args: Array[String]): Unit = {
    val x = 2
    println(x match { case E(1, _) | -7 => 1; case _ => 0 })
    val p: Any = P(2, 3)
    println(p match { case P(1, _) => 1; case P(a, b) => a + b; case _ => 0 })
    println((null: P) match { case P(_, _) => 1; case _ => 0 })
  }
}
