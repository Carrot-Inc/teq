// A singleton type inferred from a constant `final val` without a type stands for the constant's
// literal type, as scalac types the val (`final val one = 1` is `(one : 1)`).
case class Foo[T <: Int & Singleton](t: T)
object C:
  final val one = 1
  val c: 1 = Foo(one).t
@main def run(): Unit = println(C.c + 1)
