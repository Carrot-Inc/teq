// jars: scala-library cats-kernel cats-core cats-free monocle-core monocle-macro
//> using dep dev.optics::monocle-core:3.3.0
//> using dep dev.optics::monocle-macro:3.3.0
// monocle 3.3.0 from its jars: the optic classes and their `function` instances compiled from
// their TASTy bodies over the lean std's collections, composed with `andThen`. The `Focus`
// generated lens's type as the alias application `Lens[S, A]` where teq's reflect API gives
// the dealiased `PLens[S, S, A, A]` (README, macros).
import monocle.{Iso, Lens, Optional, Prism, Setter, Traversal}
import monocle.function.{At, Each, Index, Possible}
import scala.collection.immutable.ListMap

case class Street(name: String, number: Int)
case class Address(street: Street, city: String)
case class Person(name: String, age: Int, address: Address, nick: Option[String], tags: List[String], scores: Map[String, Int])
case class Box[A](value: A, label: String)
case class Wrapper(value: Int)

sealed trait Shape
case class Circle(radius: Int) extends Shape
case class Rect(w: Int, h: Int) extends Shape
case object Dot extends Shape

object Main:
  def main(args: Array[String]): Unit =
    val alice = Person("Alice", 30, Address(Street("Main", 12), "Paris"), Some("ali"), List("a", "b"), Map("math" -> 9, "art" -> 7))

    println("-- optics by hand")
    val streetLens = Lens[Address, Street](_.street)(s => a => a.copy(street = s))
    val numberLens = Lens[Street, Int](_.number)(n => s => s.copy(number = n))
    println(streetLens.andThen(numberLens).modify(_ + 1)(alice.address))
    val nickOptional = Optional[Person, String](_.nick)(n => p => p.copy(nick = Some(n)))
    println("" + nickOptional.getOption(alice) + " " + nickOptional.getOption(alice.copy(nick = None)))
    println("" + nickOptional.modify(_.toUpperCase)(alice).nick + " " + nickOptional.modifyOption(_.toUpperCase)(alice.copy(nick = None)))
    val circle = Prism.partial[Shape, Int] { case Circle(r) => r }(Circle(_))
    println("" + circle.getOption(Circle(1)) + " " + circle.getOption(Rect(1, 2)) + " " + circle.reverseGet(3))
    println("" + circle.modify(_ * 2)(Circle(2)) + " " + circle.modify(_ * 2)(Dot))
    val celsius = Iso[Double, Double](_ - 273.15)(_ + 273.15)
    println("" + celsius.get(300.0) + " " + celsius.reverseGet(0.0) + " " + celsius.reverse.get(10.0))
    val both = Traversal.apply2[Rect, Int](_.w, _.h)((w, h, r) => r.copy(w = w, h = h))
    println("" + both.getAll(Rect(2, 3)) + " " + both.modify(_ * 10)(Rect(2, 3)))
    val tagsTraversal = Traversal.fromTraverse[List, String]
    println("" + tagsTraversal.modify(_ * 2)(List("a", "b")) + " " + tagsTraversal.getAll(List("x")))
    val setter = Setter[List[Int], Int](f => _.map(f))
    println("" + setter.modify(_ + 1)(List(1, 2, 3)) + " " + setter.replace(0)(List(1, 2)))
    val addressLens = Lens[Person, Address](_.address)(a => p => p.copy(address = a))
    val composed = addressLens.andThen(streetLens).andThen(numberLens)
    println("" + composed.get(alice) + " " + composed.replace(1)(alice).address.street.number)
    println("" + tagsTraversal.find(_ == "b")(List("a", "b")) + " " + tagsTraversal.headOption(Nil) + " " + tagsTraversal.isEmpty(Nil) + " " + tagsTraversal.foldMap(_.length)(List("ab", "c")))

    println("-- function instances")
    println(Each.each[List[Int], Int].modify(_ + 1)(List(1, 2)))
    println(Each.each[Vector[Int], Int].getAll(Vector(3, 4)))
    println("" + Each.each[Option[Int], Int].modify(_ * 2)(Some(21)) + " " + Each.each[Option[Int], Int].getAll(None))
    println(Each.each[Map[String, Int], Int].modify(_ + 100)(Map("a" -> 1, "b" -> 2)))
    println(Each.each[ListMap[String, Int], Int].modify(_ - 1)(ListMap("z" -> 1, "y" -> 2)))
    println(At.at[Map[String, Int], String, Option[Int]]("a").replace(None)(Map("a" -> 1, "b" -> 2)))
    println(At.at[ListMap[String, Int], String, Option[Int]]("c").replace(Some(3))(ListMap("b" -> 2, "a" -> 1)))
    println("" + At.at[Set[Int], Int, Boolean](3).replace(true)(Set(1)) + " " + At.at[Set[Int], Int, Boolean](1).replace(false)(Set(1, 2)))
    println(At.remove[Map[String, Int], String, Int]("b")(Map("a" -> 1, "b" -> 2)))
    println("" + Index.index[List[String], Int, String](0).replace("first")(List("a", "b")) + " " + Index.index[List[String], Int, String](9).replace("x")(List("a")))
    println("" + Index.index[Vector[Int], Int, Int](1).getOption(Vector(5, 6)) + " " + Index.index[Vector[Int], Int, Int](-1).getOption(Vector(5, 6)))
    println("" + Index.index[Map[String, Int], String, Int]("k").modify(_ + 1)(Map("k" -> 1)) + " " + Index.index[Map[String, Int], String, Int]("q").getOption(Map("k" -> 1)))
    println(Index.index[ListMap[String, Int], String, Int]("k").replace(7)(ListMap("k" -> 1, "j" -> 2)))
    println("" + Possible.possible[Option[Int], Int].getOption(Some(1)) + " " + Possible.possible[Either[String, Int], Int].replace(9)(Right(1)) + " " + Possible.possible[Either[String, Int], Int].replace(9)(Left("no")))
