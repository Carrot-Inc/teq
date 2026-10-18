trait T
class K(val n: Int)
object O { given Int = 1 }
object P {
  var a: Any = 0
  val t1 = (a = new Object {})
  val t2 = (a = new Object { def f = 1 })
  val t3 = (a = new K(1) with T {})
  val t4 = (a = new T { self => })
  val t5 = (a = { import scala.math.*; 1 })
  val t6 = (a = { import scala.math.{max, min as mn}; 1 })
  val t7 = (a = { import scala.math.max, scala.math.min; 1 })
  val t8 = (a = { import O.given; 1 })
  val t9 = (a = (/*c*/ (1 + 2)))
}
