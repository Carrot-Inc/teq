// `CheckNonCyclicMap` over every branch of `mapOver` (the review of `4f8b8bb39`, item 4): a refinement member's
// information as one type (a `TypeBounds`' lower bound flipped, a `TypeAlias` at variance 0, a method's parameters
// flipped and its result not), a match type's cases at the variance restored as `MatchCase[Pat, +Body]`'s arguments
// with their binders' infos at 0, a match alias application's constructor checked (`MatchAlias`: variance 0, `match`)
// and its arguments at its parameters' variances, an object's abstract member's bounds.
// An invariant member's argument in a lower bound is the cycle (the review of `cf6cad6c1`'s MemberVarianceControl, a
// class's and an object's, the object's printed without its prefix, `proposed-errors/object_member_alias_prefix.scala`).
// expect: 22:13: error: illegal cyclic type reference: lower bound AnyRef{type T >: List[List[B]]} of type B refers back to the type itself
// expect: 24:13: error: illegal cyclic type reference: lower bound AnyRef{type T = List[List[B]]} of type B refers back to the type itself
// expect: 25:13: error: illegal cyclic type reference: lower bound AnyRef{def m(x: List[List[B]]): Int} of type B refers back to the type itself
// expect: 30:13: error: illegal cyclic type reference: lower bound Elem[B] of type B refers back to the type itself
// expect: 31:13: error: illegal cyclic type reference: lower bound Elem[B] of type B refers back to the type itself
// expect: 37:13: error: illegal cyclic type reference: lower bound PC#F[List[B]] of type B refers back to the type itself
// expect: 38:13: error: illegal cyclic type reference: lower bound
// absent: binder_cycle_map_branches.scala:23:
// absent: binder_cycle_map_branches.scala:26:
// absent: binder_cycle_map_branches.scala:27:
// absent: binder_cycle_map_branches.scala:28:
// absent: binder_cycle_map_branches.scala:29:
// absent: binder_cycle_map_branches.scala:32:
// absent: binder_cycle_map_branches.scala:33:
type RL1 = [B >: AnyRef { type T >: List[List[B]] }] =>> Int
type RL2 = [B <: AnyRef { type T = List[List[B]] }] =>> Int
type RL3 = [B >: AnyRef { type T = List[List[B]] }] =>> Int
type RM1 = [B >: AnyRef { def m(x: List[List[B]]): Int }] =>> Int
type RM2 = [B >: AnyRef { def m(x: Int): List[List[B]] }] =>> Int
type MB1 = [B <: (Int match { case List[t] => List[List[B]] })] =>> Int
type MB2 = [B >: (Int match { case List[t] => List[List[B]] })] =>> Int
type AA1 = [B <: Elem[B]] =>> Int
type AB1 = [B >: Elem[B]] =>> Int
type AC1 = [B >: Elem[List[List[B]]]] =>> Int
type Elem[X] = X match { case List[t] => Elem[t]; case _ => X }
type DB1 = [B <: O.T] =>> Int
object O { type T <: Int }
trait PC { type F[X] }
object QC { type F[X] }
type MC1 = [B >: PC#F[List[B]]] =>> Int
type MC2 = [B >: QC.F[List[B]]] =>> Int
