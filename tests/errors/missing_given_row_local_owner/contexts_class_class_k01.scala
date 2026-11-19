class C01() {
  val r01 = { def call(using need: Numeric[String]): Int = 1; call }
}
// A row: a local definition's missing given is said without its location (no effective owner of a local in this compiler yet), where scalac 3.8.4 names the owner past its weak ones: a strict val, a pattern val, a var; the expectations are scalac's lines.
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_class_class_k01.scala:2:67: error: No given instance of type Numeric[String] was found for parameter need of method call in class C01
// teq: --std=scala-library
// expect: 92 errors found
