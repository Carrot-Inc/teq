// Adapted from scala3 tests/pos/automatic-tupling-of-function-parameters.scala (Apache-2.0, see tests/scala3/README.md); replaced: arities above 5 dropped, a main that prints added.
object Test:
  val f2: Tuple2[Int, String] => Int = (x, y) => x
  val g2: Tuple2[Int, String] => Int = _ + _.length
  type T2 = Tuple2[Int, Int]
  val h2: T2 => Int = (x1, x2) => 2
  val f3: Tuple3[Int, Int, Int] => Int = (x1, x2, x3) => 3
  val g3: Tuple3[Int, Int, Int] => Int = _ + _ + _
  val f5: Tuple5[Int, Int, Int, Int, Int] => Int = (x1, x2, x3, x4, x5) => 5
  val g5: Tuple5[Int, Int, Int, Int, Int] => Int = _ + _ + _ + _ + _

@main def main(): Unit =
  println(Test.g2((1, "ab")))
  println(Test.g3((1, 2, 3)))
  println(Test.g5((1, 2, 3, 4, 5)))
