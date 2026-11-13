// What `ValueOf` and the singleton type test of a path through an export initialise. The value of a
// forwarder of a constant (a final val's, a plain val's of a literal type, a literal lazy val's, a literal
// given's) is the constant: neither the exporting object nor what stands behind the forwarder is initialised
// by it, where the forwarder's own call and the type test initialise both. A lazy val whose type is no
// constant is initialised by the first read, once. A lazy val on the path's prefix is evaluated, but not
// under an inline val's forwarder, whose value is the constant whatever the path. A type test
// an earlier case keeps from being reached initialises nothing; one that is reached initialises the exporting
// object, though the forwarder is of a constant.
package paths

object Fixed:
  final val seven: 7 = 7
  inline val three = 3

object Showing:
  println("showing-init")
  export Fixed.seven

object Plainly:
  println("plainly-init")
  val six: 6 = 6

object Passing:
  println("passing-init")
  export Plainly.six

object Lazily:
  lazy val literal: 7 = { println("literal-init"); 7 }
  lazy val wide: Int = { println("wide-init"); 8 }

object Late:
  export Lazily.{literal, wide}

object Givens:
  given chosen: 7 = { println("given-init"); 7 }

object Shared:
  export Givens.given

class Carrier:
  export Fixed.{seven, three}

object Keeping:
  lazy val carrier: Carrier = { println("carrier-init"); Carrier() }

object Waiting:
  lazy val carrier: Carrier = { println("waiting-init"); Carrier() }

object Untouched:
  println("untouched-init")
  export Fixed.seven

object Tested:
  println("tested-init")
  export Fixed.seven

def kind(x: Any): String = x match
  case _: Late.literal.type => "literal"
  case _ => "other"

def first(x: Any): String = x match
  case _: String => "string"
  case _: Untouched.seven.type => "untouched"
  case _ => "other"

def tested(x: Any): String = x match
  case _: Tested.seven.type => "tested"
  case _ => "other"

@main def run(): Unit =
  println("start")
  println(valueOf[Showing.seven.type])
  println("then the call")
  println(Showing.seven)
  println(valueOf[Late.literal.type])
  println("then the test")
  println(kind(7))
  println(valueOf[Late.wide.type])
  println(valueOf[Late.wide.type])
  println(valueOf[Keeping.carrier.seven.type])
  println(Keeping.carrier.seven)
  println(valueOf[Shared.chosen.type])
  println(valueOf[Passing.six.type])
  println("then its call")
  println(Passing.six)
  println(valueOf[Waiting.carrier.three.type])
  println(first("s"))
  println(tested(7))
  println("end")
