// A local trait inside a local class's method, its override's second-clause default beside the body's `O.this`:
// scalac prints 42 62. The readers' contract: tests/modules/efpure/a/A.scala.
@main def run(): Unit = {
  println(Lib.make(30).m(2)())
  println(Lib.make(50).m(2)())
}
