// jars: scala-library abi-callbacks-lib
// std: scala-library
// Enum statics as scalac writes them: `values`, `valueOf` and `fromOrdinal` in the companion with
// static forwarders in the enum class, found by reflection; a Scala enum over `java.lang.Enum`;
// a jar enum's values read from its companion's static fields.
import abi.{Level, Reflect}
enum Size extends java.lang.Enum[Size]:
  case Small, Large
enum Color:
  case Red, Green
enum Shape:
  case Dot
  case Box(w: Int)
@main def run(): Unit =
  println(Size.Small.ordinal)
  println(Size.values.toList)
  println(Size.valueOf("Large"))
  println(Reflect.javaEnum(Size.Large))
  println(Level.High)
  println(Reflect.values("Color"))
  println(Reflect.valueOf("Color", "Green"))
  println(Reflect.fromOrdinal("Color", 0))
  println(Reflect.fromOrdinal("Shape", 0))
  println(Reflect.values("Size"))
  println(Color.Red.ordinal + Shape.Box(1).ordinal)
