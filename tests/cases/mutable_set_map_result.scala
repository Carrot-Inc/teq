// A mutable set and a mutable map are their own builders: `result()` is the collection itself
// (zio-test counts the fibers it collects so).
import scala.collection.mutable

@main def run(): Unit =
  val set = new mutable.HashSet[Int]()
  set += 1
  set.addOne(2)
  set += 1
  val built = set.result()
  println(built.size + " " + (built eq set) + " " + built.toList.sorted)
  val map = mutable.HashMap.empty[String, Int]
  map("a") = 1
  map.addOne("b" -> 2)
  println(s"${map.result().toList.sorted} ${map.result() eq map}")
