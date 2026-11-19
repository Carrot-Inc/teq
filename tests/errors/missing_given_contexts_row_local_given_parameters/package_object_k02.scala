package q02
package object po02 {
  val (r02, s02) = ({ given call(using need: Numeric[String]): Int = 1; call }, 2)
}
// scalac 3.8.4's line whole, at one column less (the generated contexts by owners).
// expect: package_object_k02.scala:3:77: error: No given instance of type Numeric[String] was found for parameter need of given instance call in package q02.po02
