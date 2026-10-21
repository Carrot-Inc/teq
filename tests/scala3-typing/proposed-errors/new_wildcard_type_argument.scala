// A wildcard argument of a class instantiated (`new Box[? >: Int]`, an annotation's class alike): scalac 3.8.4's
// E084 `Type argument must be fully defined`, which teq does not report (master's). Met by the review of `4f8b8bb39`'s
// item 1 (a binder's annotation `@annC[? <: A]`, its E046 as scalac's beside it): a part of its own.
// expect: 7:7: error: Type argument must be fully defined
// expect: 9:17: error: Type argument must be fully defined
class annV[+T] extends scala.annotation.StaticAnnotation
@annV[? >: Int] def f = 1
class Box[+T]
val b = new Box[? >: Int]
