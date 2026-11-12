// jars: javafix
// A Java signature naming a class of a `scala` package that scala-library does not hold (`fix.StubSvc.elem():
// scala.xml.Elem`, whose class file the fixtures leave out): the class is the stub
// dotty's ClassfileParser takes from `requiredClass`, as scalac 3.8.4 reports it.
// expect: error: Bad symbolic reference. A signature
// expect: refers to Elem/T in package scala.xml which is not available.
@main def run(): Unit =
  println(fix.StubSvc.ok())
  val e = fix.StubSvc.elem()
  println(e == null)
