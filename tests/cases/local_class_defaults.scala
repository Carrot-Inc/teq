// Constructor defaults of local classes, a constant one and one reading what the class captures: scalac's
// getters are its local companion's (`Desugar.classDef`, a `LazyRef`-held local module after `LambdaLift`),
// teq's static methods of the class taking the captures first; no module reaches either, and the values,
// and when they are evaluated, are the same on every target.
object Main {
  def plain(): Int = { class Item(val x: Int = 7); new Item().x }
  def captured(n: Int): Int = {
    var reads = 0
    class Item(val x: Int = { reads += 1; n })(val y: Int = x * 2)
    val i = new Item()()
    val j = new Item(1)()
    i.x + i.y + j.y + reads * 100
  }
  def main(args: Array[String]): Unit = {
    println(plain())
    println(captured(3))
  }
}
