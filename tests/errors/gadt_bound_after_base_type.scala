sealed trait E[A]
final case class I() extends E[List[Int]]
object G:
  def g01[T <: List[Any]](e: E[T], x: T): List[Int] = e match
    case I() => x
  def g06[T <: List[Any]](e: E[T], x: T): List[Int] = e match
    case _: I => x
  def g07[T <: Seq[Any]](e: E[T], x: T): List[Int] = e match
    case I() => x
// `tryBaseType` (TypeComparer.scala 944-973): where the left side's class has a base type of the right side's class,
// `checkBase` decides, and no `fourthTry` reaches `compareGADT`: `T <: List[Any]` under the pattern's GADT bound
// `T = List[Int]` is compared through `T`'s upper bound's base type `List[Any]`, which is not below `List[Int]`, so
// scalac 3.8.4 rejects `x` in `g01` and `g06` (E007); `T <: Seq[Any]` has no base type of `List` and the GADT bound
// decides
// (`g07` accepted).
// expect: gadt_bound_after_base_type.scala:5:17: error: Found:    (x : T)
// expect: gadt_bound_after_base_type.scala:7:18: error: Found:    (x : T)
// expect: 2 errors found
// expect: Required: List[Int]
// expect: Required: List[Int]
