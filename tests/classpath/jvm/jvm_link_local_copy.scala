// jars: scala-library abi-callbacks-lib
// std: scala-library
// `copy` and its defaults on a local case class, found by reflection.
import abi.Reflect
@main def run(): Unit =
  case class L(v: Int, w: String = "w")
  val k = 7
  case class C(v: Int):
    def plus = v + k
  println(Reflect.copy(L(1), 4, "z"))
  println(Reflect.copyDefault(L(1), 2))
  println(C(1).plus)
