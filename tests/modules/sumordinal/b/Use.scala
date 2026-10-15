import scala.deriving.Mirror
import sumord.*

@main def run(): Unit =
  val m = summon[Mirror.SumOf[S]]
  println(m.ordinal(A(1)))
  println(m.ordinal(B))
  println(m eq S)
