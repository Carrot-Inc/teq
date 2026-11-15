// An anonymous class takes its creation, its own parameter list and its parent call from the constructor the typer
// selected for its superclass: a secondary constructor of a class nested in a class takes the enclosing instance
// first, as every constructor of such a class does.
class Api(val base: Int):
  class Second(val k: Int):
    def this() = this(3)
    def this(a: Int, b: Int) = this(a + b)
    def sum: Int = base + k

@main def main(): Unit =
  val api = new Api(10)
  println((new api.Second() { override def sum: Int = super.sum * 2 }).sum)
  println((new api.Second(4, 5) { override def sum: Int = super.sum * 2 }).sum)
  println((new api.Second(7) { override def sum: Int = super.sum * 2 }).sum)
  println(new api.Second().sum + new api.Second(1, 1).sum)
