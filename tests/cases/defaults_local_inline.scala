// A local inline method's ordinary getters are local functions beside it, with its captures, as a
// member's are methods of its class (`Desugar.addDefaultGetters` makes them for a local method
// too): a captured var read when the getter runs, an earlier argument passed to a later getter, a
// by-name parameter's getter returning the value, a lambda that calls them later, a method's
// receiver.
def mark(s: String): Int = { print(s); 2 }
class Box(val k: Int) {
  def run(): Int = {
    inline def scaled(a: Int = k * 10)(b: Int = a + 1): Int = a + b
    scaled()() + scaled(1)()
  }
}
@main def main(): Unit = {
  var n = 3
  inline def local(a: Int = mark("A") + n)(b: Int = mark("B")): Int = a + b
  println(local()())
  n = 5
  println(local()(mark("C")))
  println(local(10)(20))
  inline def twice(a: => Int = mark("L") * n): Int = a + a
  println(twice())
  val f = () => local()()
  n = 7
  println(f())
  println(new Box(4).run())
}
