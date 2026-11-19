enum E12 { case A12 }
object E12 {
  val r12 = for (i <- List({ def call(using need: Numeric[String]): Int = 1; call })) yield i
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_enum_companion_enum_companion_k12.scala:3:82: error: No given instance of type Numeric[String] was found for parameter need of method call in object E12
