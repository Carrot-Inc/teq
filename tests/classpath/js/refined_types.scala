// jars: scala-library refined
//> using dep eu.timepit::refined:0.11.4
// refined 0.11.4 from its jar on Scala 3: `Refined` is an opaque type, `refineV` and the
// `RefinedTypeOps` companions validate at run time through the `Validate` instances, whose
// bounds are literal type arguments (`Greater[0]`, `Interval.Closed[0, 3]`, `MaxSize[3]`,
// `MatchesRegex["..."]`) read back by the inline `WitnessAs` givens; `auto.autoUnwrap` is the
// implicit conversion to the underlying value (this version has no literal refinement macro).
import eu.timepit.refined.api.{Refined, RefType, Validate}
import eu.timepit.refined.auto.autoUnwrap
import eu.timepit.refined.boolean.{And, Not, Or}
import eu.timepit.refined.collection.{MaxSize, MinSize, NonEmpty, Size}
import eu.timepit.refined.numeric.{Greater, GreaterEqual, Interval, Less, Negative, NonNegative, Positive}
import eu.timepit.refined.refineV
import eu.timepit.refined.string.{MatchesRegex, StartsWith, Trimmed}
import eu.timepit.refined.types.all.{NonEmptyString, NonNegInt, PosInt}
import eu.timepit.refined.types.numeric.{NonNegBigDecimal, NonNegLong, PosBigDecimal, PosDouble}

case class Price(amount: NonNegBigDecimal, label: NonEmptyString)
case class Limits(min: Int Refined GreaterEqual[1], max: Int Refined Interval.Closed[1, 100])

object Main:
  def total(n: Int): Int = n * 2

  def main(args: Array[String]): Unit =
    println("-- refineV")
    println(refineV[Positive](5))
    println(refineV[Positive](-5))
    println(refineV[Positive](0))
    println(refineV[NonNegative](0))
    println(refineV[Negative](-1))
    println(refineV[Greater[10]](11))
    println(refineV[Greater[10]](10))
    println(refineV[Less[3]](5))
    println(refineV[Interval.Closed[1, 5]](3))
    println(refineV[Interval.Closed[1, 5]](6))
    println(refineV[Interval.Open[1, 5]](1))
    println(refineV[NonEmpty]("abc"))
    println(refineV[NonEmpty](""))
    println(refineV[NonEmpty](List(1)))
    println(refineV[MaxSize[3]](List(1, 2, 3)))
    println(refineV[MaxSize[3]](List(1, 2, 3, 4)))
    println(refineV[MaxSize[3]]("ab"))
    println(refineV[MinSize[2]]("a"))
    println(refineV[Size[Greater[1]]](Vector(1, 2)))
    println(refineV[MatchesRegex["[a-z]+@[a-z]+"]]("me@home"))
    println(refineV[MatchesRegex["[a-z]+@[a-z]+"]]("nope"))
    println(refineV[StartsWith["ab"]]("abc"))
    println(refineV[Trimmed](" x "))
    println(refineV[And[Positive, Less[10]]](5))
    println(refineV[And[Positive, Less[10]]](50))
    println(refineV[Or[Negative, Greater[10]]](5))
    println(refineV[Not[Positive]](0))
    println(refineV[Positive](2.5))
    println(refineV[Positive](-2L))
    println(refineV[Positive](BigDecimal("0.5")))
    println(refineV[Positive].unsafeFrom(7))

    println("-- companions")
    println(PosInt.from(3))
    println(PosInt.from(0))
    println(NonNegInt.from(0))
    println(NonNegInt.from(-1))
    println(NonNegLong.from(-1L))
    println(PosDouble.from(2.5))
    println(NonNegBigDecimal.from(BigDecimal("12.50")))
    println(NonNegBigDecimal.from(BigDecimal("-0.01")))
    println(PosBigDecimal.from(BigDecimal(0)))
    println(NonEmptyString.from("hi"))
    println(NonEmptyString.from(""))
    println(PosInt.unapply(5))
    println(PosInt.unapply(-5))
    println(NonEmptyString.unsafeFrom("x"))
    println(PosInt.unsafeFrom(2))
    try
      PosInt.unsafeFrom(-2)
      println("no failure")
    catch
      case e: IllegalArgumentException => println("IllegalArgumentException: " + e.getMessage)
    println(PosInt.MinValue.value + " " + PosInt.MaxValue.value + " " + NonNegInt.MinValue.value)
    val nn: NonNegInt = NonNegInt.unsafeFrom(4)
    val doubled: Int = nn.value * 2
    println(doubled)

    println("-- Refined")
    val amount: NonNegBigDecimal = Refined.unsafeApply(BigDecimal("3.10"))
    val label: NonEmptyString = Refined.unsafeApply("book")
    val price = Price(amount, label)
    println(price)
    println(price.amount.value + price.amount.value)
    println(price.label.value.length)
    val limits = Limits(Refined.unsafeApply(1), Refined.unsafeApply(50))
    println(limits)
    println(limits.min.value + limits.max.value)
    val Refined(raw) = price.amount
    println(raw)
    println(refineV[Positive](3).map(_.value))
    println(refineV[Positive](3).map(RefType[Refined].unwrap(_)))
    println(RefType[Refined].refine[Positive](9))
    println(RefType.applyRef[PosInt](5))
    println(RefType.applyRef[PosInt](-5))
    println(price.amount == Refined.unsafeApply[BigDecimal, NonNegative](BigDecimal("3.10")))
    println(List[PosInt](PosInt.unsafeFrom(2), PosInt.unsafeFrom(1)).map(_.value).sorted)

    println("-- auto")
    val n: PosInt = PosInt.unsafeFrom(21)
    println(total(n))
    val s: String = NonEmptyString.unsafeFrom("auto")
    println(s.toUpperCase)

    println("-- Validate")
    val v = Validate[Int, Positive]
    println("" + v.isValid(1) + " " + v.isValid(-1) + " " + v.showExpr(4) + " " + v.showResult(4, v.validate(4)) + " " + v.showResult(-4, v.validate(-4)))
    val v2 = Validate[String, NonEmpty]
    println(v2.showResult("", v2.validate("")))
    val v3 = Validate[List[Int], MaxSize[2]]
    println(v3.showResult(List(1, 2, 3), v3.validate(List(1, 2, 3))))
