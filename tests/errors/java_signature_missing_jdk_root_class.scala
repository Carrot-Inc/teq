// jars: javafix
// A Java signature naming a class of a JDK root that the JDK does not hold (`fix.StubSvc.provider():
// javax.inject.Provider[String]`, whose class file the fixtures leave out): the class is the stub
// dotty's ClassfileParser takes from `requiredClass` (`requiredSymbol`'s `newStubSymbol`)
// for every package, as scalac 3.8.4 reports it.
// expect: error: Bad symbolic reference. A signature
// expect: refers to Provider/T in package javax.inject which is not available.
@main def run(): Unit =
  println(fix.StubSvc.ok())
  val p = fix.StubSvc.provider()
  println(p == null)
