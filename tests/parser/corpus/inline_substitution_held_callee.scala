// teq: --inline-substitution
// A call in a body the expansion by substitution walks, of a method whose body the definition
// check holds back (a local class): the call is expanded by the retype path with the walk's typed
// arguments, the walk going on after it. `outer[3]` is 3, as in
// scalac 3.8.4.
inline def held(x: Int): Int =
  class C(val n: Int)
  new C(x).n
inline def outer[N <: Int]: Int =
  val x = scala.compiletime.constValue[N]
  held(x)
@main def run(): Unit = println(outer[3])
