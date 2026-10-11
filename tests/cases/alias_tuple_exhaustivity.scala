// A tuple whose element's type is an applied alias of a sealed type (`EitherPricingError[A] =
// Either[PricingError, A]`), the scrutinee's type an alias of the tuple: the space engine splits the
// element through the alias (dotty's `Space.decompose`, `rec(tp.dealias)`), so the two cases cover
// it and no warning is given, as scalac gives none.
sealed trait PricingError
case class Missing(name: String) extends PricingError
case class Id(value: String)
case class Price(v: Int)
type EitherPricingError[A] = Either[PricingError, A]
type Entry = (Id, EitherPricingError[Price])
object Main:
  def main(args: Array[String]): Unit =
    val entries: List[Entry] = List((Id("a"), Left(Missing("x"))), (Id("b"), Right(Price(2))))
    val (errors, values) = entries.partitionMap {
      case (_, Left(error)) => Left(error)
      case (id, Right(value)) => Right(id -> value)
    }
    println((errors, values.toMap))
