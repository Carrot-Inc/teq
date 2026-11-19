trait G06
given gi06: G06 with {
  lazy val r06 = { def call(using need: Numeric[String]): Int = 1; call }
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: given_instance_k06.scala:3:72: error: No given instance of type Numeric[String] was found for parameter need of method call
// teq: --std=scala-library
// expect: 7 errors found
