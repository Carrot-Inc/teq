// jars: javafix
// The stub of a class a Java signature names (`javax.inject.Provider`, which neither the JDK nor the class path holds)
// is never entered in its package (`requiredSymbol`'s `newStubSymbol`), so the program's own reference to the class
// finds no member: scalac 3.8.4 reports the stub, then the selection's E008.
// expect: error: Bad symbolic reference. A signature
// expect: refers to Provider/T in package javax.inject which is not available.
// expect: error: type Provider
@main def run(): Unit =
  val p = fix.StubSvc.provider()
  val q: javax.inject.Provider[String] = p
  println(q == null)
