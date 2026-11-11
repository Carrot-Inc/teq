// jars: javafix
// A Java method whose type parameter is bounded by a class the class path does not hold (`fix.StubMembers.bounded[T <:
// StubGone]()`): its completion reads the bound (`sig2typeBounds`, through the result's `dependencyStatus`), and the
// stub reports its `Bad symbolic reference` there, as the method is selected, as scalac 3.8.4 reports it.
// expect: error: Bad symbolic reference. A signature
// expect: refers to StubGone/T in package fix which is not available.
@main def run(): Unit =
  val b: Any = fix.StubMembers.bounded()
  println(b == null)
