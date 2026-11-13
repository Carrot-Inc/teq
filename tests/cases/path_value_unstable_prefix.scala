// A path through a member that is no stable value (an inline given whose type is a singleton) is the path
// that type names: its value is read with no call of the member, in `ValueOf` and in a singleton type test,
// where the member's own call runs its body. A constant's forwarder on such a path is the constant, the
// object not initialised.
package paths

object Fixed:
  final val five = 5

class Crate:
  val n = 7
  export Fixed.five

object Shelf:
  val crate = Crate()
  inline given handle: crate.type = { println("called"); crate }

object Rack:
  println("rack-init")
  val crate = Crate()
  inline given grip: crate.type = { println("gripped"); crate }

def held(x: Any): Boolean = x match
  case _: Shelf.handle.n.type => true
  case _ => false

@main def run(): Unit =
  println(valueOf[Shelf.handle.n.type])
  println(Shelf.handle.n)
  println(s"${held(7)} ${held(8)}")
  println(valueOf[Rack.grip.five.type])
  println("then")
  println(valueOf[Rack.grip.n.type])
