// A binder's completion as dotty's `completeFrom` over `TypeDefCompleter` has it (the review of `b23cfe2a3`, its table
// pass): its annotations' classes typed while it is `Touched`, its bounds while `Provisional`, `abstracted`'s read of the
// lower bound, `checkNonCyclic` while `Checking`; a binder is read where `isRef`, `hasSameKindAs`, `stripToCore`,
// `isLambdaSub` and `hasUnknownMembers` read it; a cycle E046 at the tree whose read met it, or E140 at the binder
// whose bounds name it back. scalac 3.8.4 gives these sixteen errors; the arguments are typed after the completion and
// the F-bounds and an earlier binder are legal.
// absent: binder_completion_reads.scala:31:
// absent: binder_completion_reads.scala:32:
// absent: binder_completion_reads.scala:33:
// absent: binder_completion_reads.scala:34:
// absent: binder_completion_reads.scala:35:
// absent: binder_completion_reads.scala:36:
class ann[T] extends scala.annotation.StaticAnnotation
class ann2(x: Any) extends scala.annotation.StaticAnnotation
type Siblings = [@ann[B] A, @ann[A] B] =>> Int
type Recursive = [@ann[B] A, @ann[C] B, @ann[A] C] =>> Int
type UnannotatedFirst = [A <: B, @ann[A] B] =>> Int
type Chain = [@ann[B] A, B <: C, C <: A] =>> Int
type Nested = [@ann[B] A, B <: List[A]] =>> Int
type Lower = [@ann[B] A, B >: A] =>> Int
type Upper = [@ann[B] A, B <: A] =>> Int
type Projection = [@ann[A#X] A <: { type X }] =>> Int
type Refined = [@ann[A { type X }] A] =>> Int
type Matched = [@ann[A match { case Int => Int }] A] =>> Int
type Annotated = [@ann[A @unchecked] A] =>> Int
type Plain = [B <: C, C <: B] =>> Int
type Self = [B <: B] =>> Int
type Lowers = [B >: C, C >: B] =>> Int
val literal = [B <: C, C <: B] => (b: B) => b
type Fun = [B <: C, C <: B] => B => Int
type Args1 = [@ann2(??? : List[A]) A] =>> List[A]
type Args2 = [@ann2((x: List[A]) => x) A] =>> List[A]
type Args3 = [@ann2(??? : (A, Int)) A] =>> List[A]
type FBound = [B <: List[B]] =>> Int
type FBounds = [B <: List[C], C <: B] =>> Int
type Earlier = [A, @ann[A] B] =>> Int
