// jars: scala-library magnolia
// A loaded class with two constructors, applied to written type arguments, the first of a higher kind: both
// take the type arguments, and scalac keeps among them those whose bounds the arguments meet
// (Applications.resolveOverloaded, TypeOps.boundsViolations). The primary constructor's parameters are the
// class's own, whose `[_] =>> Any` bound is no bound.
import magnolia1.*

trait Derivation:
  type Typeclass[T]
  def primaryOverMember[A, S](info: TypeInfo, typeclass: CallByNeed[Typeclass[S]], isType: A => Boolean, asType: A => S & A): SealedTrait.Subtype[Typeclass, A, S] =
    new SealedTrait.Subtype[Typeclass, A, S](info, IArray.empty[Any], IArray.empty[Any], IArray.empty[Any], false, 0, typeclass, isType, asType)

object Fixed:
  def primary[A, S](info: TypeInfo, typeclass: CallByNeed[List[S]], isType: A => Boolean, asType: A => S & A): SealedTrait.Subtype[List, A, S] =
    new SealedTrait.Subtype[List, A, S](info, IArray.empty[Any], IArray.empty[Any], IArray.empty[Any], false, 0, typeclass, isType, asType)
  def secondary[A, S](info: TypeInfo, typeclass: CallByNeed[List[S]], isType: A => Boolean, asType: A => S & A): SealedTrait.Subtype[List, A, S] =
    new SealedTrait.Subtype[List, A, S](info, IArray.empty[Any], IArray.empty[Any], false, 0, typeclass, isType, asType)
  def inferred[A, S](info: TypeInfo, typeclass: CallByNeed[List[S]], isType: A => Boolean, asType: A => S & A): SealedTrait.Subtype[List, A, S] =
    new SealedTrait.Subtype(info, IArray.empty[Any], IArray.empty[Any], IArray.empty[Any], false, 0, typeclass, isType, asType)
