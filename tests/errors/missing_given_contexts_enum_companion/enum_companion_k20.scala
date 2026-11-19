enum E20 { case A20 }
object E20 {
  { def call(using need: Numeric[String]): Int = 1; call }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: enum_companion_k20.scala:3:57: error: No given instance of type Numeric[String] was found for parameter need of method call
