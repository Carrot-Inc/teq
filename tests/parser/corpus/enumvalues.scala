enum Level:
  case Low, Mid, High

  def next: Level = Level.fromOrdinal((ordinal + 1) % Level.values.length)

enum Shape:
  case Circle(r: Double)
  case Unit

@main def main(): Unit =
  println(Level.values.toList)
  println(Level.values.length)
  println(Level.valueOf("Mid"))
  println(Level.fromOrdinal(2))
  println(Level.High.next)
  println(Level.values.map(_.ordinal).toList)
  println(Level.values.find(_.toString == "High"))
  val copy = Level.values
  copy(0) = Level.High
  println(Level.values.toList.head)
  println(Shape.Unit.ordinal)
