// jars: javafix
// A Java method whose throws clause names a class the class path does not hold (`fix.StubMembers.thrower() throws
// StubGoneEx`): its completion looks each exception's class up (`AttributeCompleter.complete`'s `getClassSymbol`,
// ClassfileParser.scala:913-915), and the stub reports its `Bad symbolic reference` there, as scalac 3.8.4 reports it.
// expect: error: Bad symbolic reference. A signature
// expect: refers to StubGoneEx/T in package fix which is not available.
@main def run(): Unit =
  fix.StubMembers.thrower()
  println("done")
