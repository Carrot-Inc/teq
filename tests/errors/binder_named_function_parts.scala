// A named function type's later parameter and its result while a binder completes (the review of `cf6cad6c1`,
// item 2): the second parameter's `valOrDefDefSig` (E046 at `y`), and a result applied to the binder (`List[A]`'s
// `preCheckKinds`, E046 at it), as scalac 3.8.4 gives them.
// expect: 7:35: error: Cyclic reference involving type A
// expect: 8:33: error: Cyclic reference involving type A
class ann[T] extends scala.annotation.StaticAnnotation
type SecondParam = [@ann[(x: Int, y: A) => Int] A] =>> Int
type Result = [@ann[(x: Int) => List[A]] A] =>> Int
