abstract class Shape:
  val name: String
  val sides: Int
  println(s"Shape body sees $name with $sides sides")
  val label = name.toUpperCase + sides

class Polygon(val name: String, val sides: Int) extends Shape:
  println(s"Polygon body, label $label")

class Square extends Polygon("square", 4):
  println("Square body")
  val area = sides * sides

object Origin extends Polygon("origin", 0):
  println("Origin body")

trait Audited:
  println("Audited body")

class Ledger(val id: Int) extends Audited:
  println(s"Ledger $id")
class SubLedger(id: Int, val tag: String) extends Ledger(id + 1):
  println(s"SubLedger $tag of $id")

@main def run(): Unit =
  val p = Polygon("tri", 3)
  println(p.label)
  val s = Square()
  println(s.label + " " + s.area)
  println(Origin.label)
  val a = new Polygon("anon", 9) { println("anon body " + label) }
  println(a.sides)
  SubLedger(1, "t")
  Ledger(5)
