// jars: scala-library abi-callbacks-lib
// std: scala-library
// The companion of a case class as scalac writes it, found by reflection: `apply`, `unapply`,
// `fromProduct` of `Mirror.Product`, the constructor's default getters, a final `MODULE$`; `copy`
// and its defaults in the case class. A case class with two parameter clauses has no mirror.
// The classes are named in code, since the dead-code pass keeps only what the program reaches.
import abi.Reflect

case class Point(x: Int, y: Int = 3)
case class Gen[A](a: A, n: String = "n")
object Gen:
  val self = Gen
  def make: Gen[Int] = Gen(1)
case class Named(first: String)(val second: Int = 7)
enum Color:
  case Red
  case Mix(a: Int)
object Holder:
  case class Inner(v: Long, c: Char)

@main def run(): Unit =
  println(Seq(Color.Mix(1), Holder.Inner(1L, 'a')))
  println(Reflect.companion("Point", 1, 2))
  println(Reflect.moduleName("Point"))
  println(Reflect.companion("Gen", "a", "b"))
  println(Gen.make)
  println(Gen.self eq Gen)
  println(Reflect.companion("Color$Mix", 4))
  println(Reflect.companion("Holder$Inner", 5L, 'c'))
  println(Reflect.constructorDefault("Point", 2))
  println(Reflect.constructorDefault("Gen", 2))
  println(Reflect.finalModule("Point"))
  println(Reflect.finalModule("Gen"))
  println(Reflect.copy(Point(1, 2), 5, 6))
  println(Reflect.copyDefault(Point(1, 2), 2))
  println(Reflect.copy(Named("x")(1), "y", 8))
