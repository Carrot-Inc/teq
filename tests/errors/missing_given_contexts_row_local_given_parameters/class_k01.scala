class C01() {
  val r01 = { given call(using need: Numeric[String]): Int = 1; call }
}
// A row: teq refuses a local given with parameters ("local givens cannot take parameters"), where scalac 3.8.4 takes it; the expectations are scalac's lines, the local given called by its name in every context by every owner.
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: class_k01.scala:2:69: error: No given instance of type Numeric[String] was found for parameter need of given instance call in class C01
// teq: --std=scala-library
// expect: 174 errors found
