class C02() {
  val (r02, s02) = ({ def call(using need: Numeric[String]): Int = 1; call }, 2)
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: contexts_class_class_k02.scala:2:75: error: No given instance of type Numeric[String] was found for parameter need of method call in class C02
