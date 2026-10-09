package mob
import mo.*
import scala.deriving.Mirror
@main def run(): Unit =
  Count.calls = 0
  val out = summon[Mirror.ProductOf[Out]].fromProduct(Input(3))
  println(out.x.toString + ":" + Count.calls)
  println(Mac.result)
