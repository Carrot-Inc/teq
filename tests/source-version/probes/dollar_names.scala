object P {
  val x$y = 1
  val `a$b` = 2
  val $ = 3
  val $$ = 4
  def f(x$z: Int) = x$z
  val (p$q, r) = (1, 2)
  def g(x: (Int, Int)) = x match { case ($, $$) => $ + $$; case (m$n, _) => m$n }
  class C$D[T$U](val k$l: Int)
  type A$B = Int
  object O$P
  val h = (y$z: Int) => y$z
  def i$j(): Unit = ()
}
