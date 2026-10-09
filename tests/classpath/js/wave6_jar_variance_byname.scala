// jars: fixtures
// targets: js interp jvm
// An opaque type of a jar keeps its parameter's variance (`opaque type Box[+A]`, as scalajs-react's
// `TagLite.Tag[+N]`), and a lambda of a jar's body whose parameter is by-name (`(fa: => F[Unit]) => ..` of
// `Attr.EventCallback2.dispatch`) takes a thunk it evaluates where it reads it.
// The expectation is scalac 3.8.4's, run over tests/tasty/src/wave6.scala and this file.
import fix.wave6.*

@main def main(): Unit =
  val b: W6Box.Box[Any] = W6Box.mk(1)
  println(b.items)
  println(W6Thunk.count())
  var n = 0
  println(W6Thunk.twice({ n += 10; n }))
