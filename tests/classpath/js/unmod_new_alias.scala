// jars: fixtures
// A library body's `new` of an alias that fixes some of its class's type parameters
// (`type Scope[P] = UmScope[Option, P]`, scalajs-react's `new Lifecycle.RenderScope`), pickled by
// Scala 3.3; the expectation is scalac 3.8.4's.
import fix.unmod.*

@main def main(): Unit =
  println(UmAliases.make(3).show)
