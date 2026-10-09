// Overloaded extension methods, one parameterless: the arguments of the call pick the
// alternative that takes them.
package extensionoverloadarity

opaque type Box = Int
object Box:
  def of(i: Int): Box = i
  extension (b: Box)
    def show: String = show("<", ">")
    def show(sep: String): String = show("", sep)
    def show(l: String, r: String): String = l + b.toString + r
@main def main(): Unit =
  val b = Box.of(1)
  println(b.show)
  println(b.show(","))
  println(Box.of(2).show("-"))
  println(b.show("[", "]"))
  println(Box.of(3).show.length)
