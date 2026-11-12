// A class or enum that is a function with a field named like
// a JavaScript function's own `name` or `length`: the field holds
// the class's value, an overridden one the subclass's, a
// case's the case's or its enum's, as scalac prints them.
enum E(val name: String) extends (Int => Int):
  def apply(i: Int): Int = i + name.length
  def get = name
  case O(override val name: String) extends E(name + "!")
  case P(x: Int) extends E("p" + x)
  case K extends E("kay")

enum L(val length: Int) extends (Int => Int):
  def apply(i: Int): Int = i * length
  def get = length
  case A(override val length: Int) extends L(length + 1)
  case B(x: Int) extends L(x * 10)

class F extends (Int => Int):
  val name: String = "f"
  def apply(i: Int): Int = i + name.length
class G(override val name: String) extends F
class H extends F

@main def run(): Unit =
  val o = E.O("a")
  println(s"${o.name} ${o.get} ${o(1)}")
  val p = E.P(3)
  println(s"${p.name} ${p.get} ${p(1)}")
  println(s"${E.K.name} ${E.K.get} ${E.K(1)}")
  val a = L.A(2)
  println(s"${a.length} ${a.get} ${a(1)}")
  val b = L.B(2)
  println(s"${b.length} ${b.get} ${b(1)}")
  println(s"${G("gg").name} ${G("gg")(1)}")
  println(s"${H().name} ${H()(1)}")
