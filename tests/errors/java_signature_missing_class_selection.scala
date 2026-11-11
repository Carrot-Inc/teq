// jars: javafix
// A Java member whose result names a class missing from the class path reports the stub's `Bad symbolic reference`
// where it is selected, whatever its value goes to (here a statement): its completion reads its result's parts
// (`MethodType#dependencyStatus`), whose stub completes there, as scalac 3.8.4's typer reports it.
// expect: error: Bad symbolic reference. A signature
// expect: refers to StubGone/T in package fix which is not available.
@main def run(): Unit =
  fix.StubRef.gone()
  println("done")
