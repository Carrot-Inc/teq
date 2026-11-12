// jars: missingclass-mixed
// A class missing from the class path that a TASTy file names first and a Java signature names after it is the stub
// for the Java signature all the same (`requiredClass`'s), so scalac 3.8.4's `Bad symbolic reference`
// is to come whatever the order the readers met it in, and on any number of threads.
// expect: error: Bad symbolic reference. A signature
// expect: refers to Gone2/T in package lib2 which is not available.
@main def run(): Unit =
  val u: Any = lib2.Uses.gone
  val g = lib2.JRef.gone()
  println(g == null)
