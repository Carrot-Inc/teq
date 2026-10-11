// expect: 9:40: error: no given instance of type (B.U : U) was found for parameter u
// expect: 1 error found
// An object nested in a trait is in the implicit scope of its own singleton type through the
// object extending the trait, but it is no given unless it is implicit.
trait A:
  object U
  def check(using u: U.type): String = "bad"
object B extends A
@main def run(): Unit = println(B.check)
