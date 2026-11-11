// jars: javafix
// A Java field whose type is a class the class path does not hold (`fix.StubMembers.field: StubGone`): its completion
// cooks the type, which asks the class reference for its type parameters (ClassfileParser.scala:64-66), and the stub
// reports its `Bad symbolic reference` there, as the field is selected, as scalac 3.8.4 reports it.
// expect: error: Bad symbolic reference. A signature
// expect: refers to StubGone/T in package fix which is not available.
@main def run(): Unit =
  val x: Any = fix.StubMembers.field
  println(x == null)
