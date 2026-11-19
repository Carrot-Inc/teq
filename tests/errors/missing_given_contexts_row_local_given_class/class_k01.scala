class C01() {
  val r01 = { given call(using need: Numeric[String]): Serializable with {}; call.hashCode }
}
// A row: teq takes a reference to a local structural given with a context parameter without that parameter's argument (no error at all), where scalac 3.8.4 refuses it; the expectations are scalac's lines, the given called by its name in every context by every owner.
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: class_k01.scala:2:82: error: No given instance of type Numeric[String] was found for parameter need of given instance call in class C01
// teq: --std=scala-library
// expect: 174 errors found
