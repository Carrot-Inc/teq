// A type definition's own clause's binders complete in turn, each one's annotations typed while it completes
// (`completerTypeParams`): its information forced then is `completeFrom`'s cycle (E046), where an applied type, a
// function type, a tuple, a union asks its kind; a later sibling forced completes then, its bound the binder E140; a
// reference that forces nothing is legal, a lambda no definition's own sees no binder (the review of `50e3a1b38`).
// expect: 19:14: error: Cyclic reference involving type A
// expect: 20:21: error: Cyclic reference involving type A
// expect: 21:15: error: Cyclic reference involving type A
// expect: 22:23: error: Cyclic reference involving type A
// expect: 23:20: error: Cyclic reference involving type A
// expect: 24:28: error: illegal cyclic type reference: upper bound A of type B refers back to the type itself
// expect: 29:23: error: type mismatch: found Null, required A
// expect: 30:23: error: type A not found
// absent: binder_completion_cycle.scala:25:
// absent: binder_completion_cycle.scala:26:
// absent: binder_completion_cycle.scala:27:
// absent: binder_completion_cycle.scala:28:
class ann[T] extends scala.annotation.StaticAnnotation
class ann2(x: Any) extends scala.annotation.StaticAnnotation
type Own = [@ann[A] A] =>> List[A]
type Nested = [@ann[List[A]] A] =>> List[A]
trait Kind[F[@ann[A] A]]
type Function = [@ann[A => Int] A] =>> List[A]
type Union = [@ann[A | Int] A] =>> List[A]
type Sibling = [@ann[B] A, B <: A] =>> Int
type SiblingFree = [@ann[B] A, B] =>> Int
type Nothing_ = [@ann2(??? : A) A] =>> List[A]
type Term = [@ann2(List.empty[A]) A] =>> List[A]
type Lambda = [@ann2((x: A) => x) A] =>> List[A]
type Subtype = [@ann2(null: A) A] =>> List[A]
type NotOwn = M[[@ann[A] A] =>> Int]
type M[G[_]] = Int
