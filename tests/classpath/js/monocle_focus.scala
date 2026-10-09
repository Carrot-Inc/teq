// jars: scala-library cats-kernel cats-core cats-free monocle-core monocle-macro
//> using dep dev.optics::monocle-core:3.3.0
//> using dep dev.optics::monocle-macro:3.3.0
// monocle 3.3.0's `Focus` macro from its jar, run in teq's interpreter through its reflect API:
// `GenLens`, `Focus(_.a.b)`, `.focus(_.x)` and the keywords `some`, `each`, `at`, `index`, `as`
// and `withDefault` on nested case classes, options, maps, lists and a case class with a type
// parameter; `GenPrism` beside them (`GenIso`, deprecated since 3.1.0, matches a quote pattern
// over a refinement type, which teq does not read).
import monocle.{Focus, Iso, Lens, Optional, Prism, Setter, Traversal}
import monocle.macros.{GenLens, GenPrism}
import monocle.syntax.all.*

case class Street(name: String, number: Int)
case class Address(street: Street, city: String, flat: Option[Int])
case class Person(name: String, age: Int, address: Address, nick: Option[String], tags: List[String], scores: Map[String, Int], spouse: Option[Person])
case class Box[A](value: A, label: String)
case class Pair[A, B](left: A, right: B)
case class Wrapper(value: Int)
case class Config(name: String, retries: Option[Int], boxes: List[Box[Int]], byName: Map[String, Box[String]])

sealed trait Shape
case class Circle(radius: Int) extends Shape
case class Rect(w: Int, h: Int) extends Shape
case object Dot extends Shape

object Main:
  def main(args: Array[String]): Unit =
    val alice = Person("Alice", 30, Address(Street("Main", 12), "Paris", Some(3)), Some("ali"), List("a", "b"), Map("math" -> 9, "art" -> 7), None)
    val bob = Person("Bob", 33, Address(Street("Side", 4), "Lyon", None), None, Nil, Map.empty, Some(alice))

    println("-- GenLens")
    val name = GenLens[Person](_.name)
    val age = GenLens[Person](_.age)
    val city = GenLens[Person](_.address.city)
    val number = GenLens[Person](_.address.street.number)
    println(s"${name.get(alice)} ${age.modify(_ + 1)(alice).age} ${city.replace("Rome")(alice).address.city}")
    println(number.modify(_ * 2)(alice).address.street)
    println(name.andThen(Lens[String, Int](_.length)(_ => s => s)).get(alice))

    println("-- Focus")
    val street = Focus[Person](_.address.street)
    println(s"${street.get(alice)} ${street.replace(Street("Rue", 1))(alice).address}")
    println(Focus[Address](_.city).get(alice.address))
    println(Focus[Box[Int]](_.value).modify(_ + 1)(Box(1, "one")))
    println(Focus[Box[String]](_.value).modify(_.toUpperCase)(Box("one", "1")))
    println(Focus[Pair[Int, String]](_.right).replace("r")(Pair(1, "l")))
    println(Focus[Pair[Int, Box[Int]]](_.right.value).modify(_ * 10)(Pair(1, Box(2, "b"))))

    println("-- some, withDefault")
    val nick = Focus[Person](_.nick.some)
    println(s"${nick.getOption(alice)} ${nick.getOption(bob)} ${nick.modify(_.toUpperCase)(alice).nick}")
    val flat = Focus[Person](_.address.flat.some)
    println(s"${flat.getOption(alice)} ${flat.replace(9)(bob).address.flat}")
    val nickOrNone = Focus[Person](_.nick.withDefault("none"))
    println(s"${nickOrNone.get(alice)} ${nickOrNone.get(bob)} ${nickOrNone.replace("bobby")(bob).nick}")
    val spouseName = Focus[Person](_.spouse.some.name)
    println(s"${spouseName.getOption(bob)} ${spouseName.getOption(alice)} ${spouseName.replace("Alicia")(bob).spouse.map(_.name)}")

    println("-- each, at, index")
    val tags = Focus[Person](_.tags.each)
    println(s"${tags.getAll(alice)} ${tags.modify(_ + "!")(alice).tags} ${tags.getAll(bob)}")
    val math = Focus[Person](_.scores.at("math"))
    println(s"${math.get(alice)} ${math.replace(Some(10))(alice).scores} ${math.replace(None)(alice).scores}")
    val art = Focus[Person](_.scores.index("art"))
    println(s"${art.getOption(alice)} ${art.modify(_ + 1)(alice).scores} ${art.getOption(bob)}")
    val second = Focus[Person](_.tags.index(1))
    println(s"${second.getOption(alice)} ${second.replace("z")(alice).tags} ${second.getOption(bob)}")
    val scoreValues = Focus[Person](_.scores.each)
    println(s"${scoreValues.getAll(alice).sorted} ${scoreValues.modify(_ * 2)(alice).scores}")
    val boxValues = Focus[Config](_.boxes.each.value)
    val config = Config("c", Some(2), List(Box(1, "a"), Box(2, "b")), Map("k" -> Box("v", "l")))
    println(s"${boxValues.getAll(config)} ${boxValues.modify(_ + 10)(config).boxes}")
    println(s"${Focus[Config](_.byName.index("k").value).getOption(config)} ${Focus[Config](_.retries.some).modify(_ + 1)(config).retries}")

    println("-- as, GenPrism")
    val circle = Focus[Shape](_.as[Circle])
    val rect = GenPrism[Shape, Rect]
    val shapes: List[Shape] = List(Circle(3), Rect(1, 2), Dot)
    println(s"${shapes.map(circle.getOption)} ${shapes.map(rect.getOption)}")
    println(s"${circle.andThen(Focus[Circle](_.radius)).modify(_ + 1)(Circle(3))} ${circle.andThen(Focus[Circle](_.radius)).modify(_ + 1)(Dot)}")
    println(rect.andThen(GenLens[Rect](_.w)).replace(7)(Rect(1, 2)))
    println(Focus[Wrapper](_.value).get(Wrapper(8)))

    println("-- focus syntax")
    println(s"${alice.focus(_.name).get} ${alice.focus(_.age).modify(_ + 5).age} ${alice.focus(_.address.street.name).replace("Rue").address.street}")
    println(s"${alice.focus(_.nick.some).getOption} ${bob.focus(_.tags.each).getAll} ${alice.focus(_.scores.at("art")).replace(None).scores}")
    println(s"${alice.focus(_.spouse.some.address.city).getOption} ${bob.focus(_.spouse.some.address.city).getOption}")
    println(config.focus(_.boxes.index(0).label).replace("first").boxes)

    println("-- typed lambdas, keyword chains, collections of optics")
    val typedName = GenLens[Person]((p: Person) => p.name)
    println(s"${typedName.get(alice)} ${GenLens[Person]((p: Person) => p.address.city).replace("Oslo")(alice).address.city}")
    val scoreOrZero = GenLens[Person]((p: Person) => p.scores).at("art").withDefault[Int](0)
    println(s"${scoreOrZero.get(alice)} ${scoreOrZero.get(bob)} ${scoreOrZero.replace(3)(bob).scores}")
    val eachTag: Setter[Person, String] = GenLens[Person](_.tags).each
    println(eachTag.modify(_.toUpperCase)(alice).tags)
    val setters = List(GenLens[Person](_.tags).each, Focus[Person](_.spouse.some.tags.each))
    println(setters.map(_.replace("z")(bob).tags))

    println("-- composed")
    val nested = GenLens[Person](_.address).andThen(GenLens[Address](_.street)).andThen(GenLens[Street](_.name))
    println(s"${nested.get(alice)} ${nested.modify(_.reverse)(alice).address.street.name}")
    val optional: Optional[Person, Int] = Focus[Person](_.spouse.some.age)
    println(s"${optional.getOption(bob)} ${optional.modify(_ + 1)(bob).spouse.map(_.age)}")
    val traversal: Traversal[Person, String] = Focus[Person](_.tags.each)
    println(traversal.foldMap(_.length)(alice))
    val lens: Lens[Person, String] = Focus[Person](_.name)
    val prism: Prism[Shape, Circle] = Focus[Shape](_.as[Circle])
    val iso: Iso[Wrapper, Wrapper] = Focus[Wrapper]()
    println(s"${lens.get(alice)} ${prism.getOption(Circle(1))} ${iso.get(Wrapper(2))}")
