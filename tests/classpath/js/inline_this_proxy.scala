// jars: fixtures
// targets: js interp jvm
// An inline method's expansion at a call site outside the trait and its companion: the anonymous class the body makes
// reads the companion's `private[pkg]` val through the trait's inline accessor, whose body reads it through the
// companion's `this`; outside the companion that `this` is the companion's module. The expectation is scalac 3.8.4's.
import fix.inlineproxy.proxyAuto

@main def main(): Unit =
  println(proxyAuto.viaClass)
