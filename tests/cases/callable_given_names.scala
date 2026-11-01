// A class or object that is a function (an instance is a JavaScript function) with a given or a lazy val named like a
// function's own `name` or `length`: the given's value, as scalac reads it. The runtime reaches such a member through
// the prototype in place of the function's own property, which it deletes, and never runs the member's getter itself.
object O1 extends (Int => Int):
  lazy val name: (String => String) = s => s + "!"
  def apply(i: Int) = i
object O2 extends (Int => Int):
  given name: String = "g"
  def apply(i: Int) = i
class C extends (Int => Int):
  given length: (Int => Int) = _ * 3
  def apply(i: Int) = i
object O3 extends (Int => Int):
  given name: Function1[Int, Int] with
    def apply(i: Int) = i + 100
  given length: Conversion[Int, String] = i => "#" + i
  def apply(i: Int) = i
enum G extends (Int => Int):
  given name: (String => String) = s => s + "?"
  def apply(i: Int) = i
  case K

@main def run(): Unit =
  println(O1.name("x"))
  println(O2.name)
  println(C().length(2))
  println(O3.name(1))
  println(O3.length(7))
  println(G.K.name("y"))
