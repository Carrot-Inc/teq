package ucpb
import ucpa.*
@main def run(): Unit =
  println(P.make(using 5).doubled)
  println(D(3).m)
  given Int = 4
  val e = E()
  println(s"${e.n} ${e.m}")
  println(F().m + G().m)
  println(P.withCtx.twice)
