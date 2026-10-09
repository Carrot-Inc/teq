// jars: fixtures
// targets: js interp jvm
// Members of a jar that `@targetName` names in its class files: overloads that erase alike and
// operators, called from the program and from the jar's own body.
import fix.tname.{Counter, Variance}

@main def main(): Unit =
  println(Variance.extract(List(3, 4)))
  println(Variance.extract(List("x", "y")))
  println(Variance.both)
  val c = new Counter(1) + new Counter(2)
  println(c)
  println(c + List(new Counter(10), new Counter(20)))
