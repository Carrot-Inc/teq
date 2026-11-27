// A polymorphic method's default of a narrower type than its parameter's (`m[A](a: A = "yes")`): scalac prints yes.
// The readers' contract: tests/modules/efpure/a/A.scala.
@main def run(): Unit = println(Narrow.run())
