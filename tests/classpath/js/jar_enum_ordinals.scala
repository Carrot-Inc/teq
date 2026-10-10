// jars: fixtures
// std: lean scala-library
// targets: js interp
// A scalac-built enum mixing value and class cases: each case's ordinal is the one scalac encodes
// (`DesugarEnums.nextOrdinal`, the order of declaration), a value case's in its `$new` or in the
// anonymous class its parent arguments make, a class case's in its `ordinal`; the jar lists the
// class cases first. Read through `scala.reflect.Enum` too.
import fix.shapes.{Color, Tree}

object Main:
  def main(args: Array[String]): Unit =
    println(List(Color.Red.ordinal, Color.Green.ordinal, Color.Custom(7).ordinal))
    println(List(Tree.Leaf.ordinal, Tree.Node(Tree.Leaf, 1, Tree.Leaf).ordinal))
    val e: scala.reflect.Enum = Color.Green
    println(e.ordinal)
