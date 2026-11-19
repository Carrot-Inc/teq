class C18() {
  val r18 = s"${ def call(using need: Numeric[String]): Int = 1; call }"
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: class_k18.scala:2:70: error: No given instance of type Numeric[String] was found for parameter need of method call in class C18
// teq: --std=scala-library
// expect: 21 errors found
// A row: teq's parser refuses a definition inside an interpolation's `${ }` ("expected an expression, found 'def'"), which scalac 3.8.4 reads as a block; the expectations are scalac's lines.
