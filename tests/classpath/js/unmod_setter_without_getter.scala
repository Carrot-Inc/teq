// jars: fixtures
// A library body calling a method named like a setter, `c.x_=(7)`, with no getter `x` beside it
// (pickled by Scala 3.3); the expectation is scalac 3.8.4's.
import fix.unmod.*

@main def main(): Unit =
  println(UmSetterUse.use(new UmSetterOnly))
