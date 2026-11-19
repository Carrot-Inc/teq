class C05() {
  var r05 = { def call(using need: Numeric[String]): Int = 1; call }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_class_class_k05.scala:2:67: error: No given instance of type Numeric[String] was found for parameter need of method call in class C05
