enum E05 { case A05 }
object E05 {
  var r05 = { def call(using need: Numeric[String]): Int = 1; call }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_enum_companion_enum_companion_k05.scala:3:67: error: No given instance of type Numeric[String] was found for parameter need of method call in object E05
