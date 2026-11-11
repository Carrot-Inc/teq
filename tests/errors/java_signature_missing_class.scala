// jars: javafix
// A Java signature naming a class the class path does not hold (`fix.StubRef.gone(): StubGone`, whose class file the
// fixtures leave out): the class is the stub dotty's ClassfileParser takes from `requiredClass` (`requiredSymbol`'s
// `newStubSymbol`), whose completion reports `Bad symbolic reference` with no position (`StubInfo#complete`), as
// scalac 3.8.4 reports it for the call that needs the class. `ok()` beside it needs nothing of the class.
// expect: error: Bad symbolic reference. A signature
// expect: refers to StubGone/T in package fix which is not available.
// expect: the classpath might be incompatible with the version used when compiling the signature.
@main def run(): Unit =
  println(fix.StubRef.ok())
  val g = fix.StubRef.gone()
  println(g == null)
