// jars: scala-library magnolia
//> using dep com.softwaremill.magnolia1_3::magnolia:1.3.18
// Magnolia's macros one by one, run in the interpreter from the jar's TASTy bodies: the
// reflection API's symbols, annotations, constructors and trees over a case class, a sealed
// trait, an enum and a case object. `repeated` of a `T*` parameter is left out: its type tree is
// not an `AnnotatedType` here (README, macros). `inheritedParamAnns` is sorted: Magnolia groups
// it through a `HashMap`, whose order the lean std's insertion-ordered `Map` does not follow.
import magnolia1.Macro

final case class note(text: String) extends scala.annotation.StaticAnnotation

@note("on the class")
case class Address(street: String, @note("field") city: String, zip: Option[String] = None, tags: String*)
sealed trait Shape
case class Circle(r: Int) extends Shape
case object Dot extends Shape
enum Color:
  case Red, Green

object Main:
  def main(args: Array[String]): Unit =
    println(Macro.paramAnns[Address])
    println(Macro.inheritedParamAnns[Address].sortBy(_._1))
    println(Macro.paramTypeAnns[Address])
    println(Macro.defaultValue[Address].map((n, d) => (n, d.map(_.apply()))))
    println(Macro.typeInfo[Address])
    println(Macro.typeInfo[Option[Int]])
    println(Macro.typeInfo[Dot.type])
    println(Macro.typeInfo[Color])
    println(Macro.typeInfo[Color.Red.type])
    println(Macro.isObject[Address])
    println(Macro.isObject[Dot.type])
    println(Macro.isEnum[Color])
    println(Macro.isEnum[Shape])
    println(Macro.isValueClass[Address])
    println(Macro.anns[Address])
    println(Macro.inheritedAnns[Circle])
    println(Macro.typeAnns[Address])
    println(Macro.paramAnns[Circle])
    println(Macro.repeated[Circle])
    println(Macro.defaultValue[Circle])
    println(Macro.paramAnns[Dot.type])
    println(Macro.anns[Color.Red.type])
