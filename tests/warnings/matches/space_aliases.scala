// The space engine splits a type through an alias, applied or not (dotty's `Space.decompose`,
// `rec(tp.dealias)`): a tuple element typed by an applied alias of `Either` is covered by `Left` and
// `Right`, a field typed by an alias of a case class takes its product, a scrutinee typed by an alias
// of `Vector` its sequence space; one case short of them is reported.
sealed trait PricingError
case class Missing(name: String) extends PricingError
case class Id(value: String)
case class Price(v: Int)
type EitherPricingError[A] = Either[PricingError, A]
type Entry = (Id, EitherPricingError[Price])
type I = Id
type V = Vector[Boolean]
def covered(e: Entry): Int = e match { case (_, Left(_)) => 1; case (_, Right(_)) => 2 }
def lambda(entries: List[Entry]) = entries.partitionMap { case (_, Left(error)) => Left(error); case (id, Right(value)) => Right(id -> value) }
def onlyLeft(e: Entry): Int = e match { case (_, Left(_)) => 1 }
def field(x: (I, Boolean)): Int = x match { case (Id(_), true) => 1 }
def fieldCovered(x: (I, Boolean)): Int = x match { case (Id(_), true) => 1; case (Id(_), false) => 2 }
def seqAlias(v: V): Int = v match { case Vector(true, _) => 1 }
def nested(x: Option[EitherPricingError[Price]]): Int = x match { case Some(Right(_)) => 1; case None => 0 }

@main def run(): Unit =
  println(covered((Id("a"), Right(Price(1)))) + fieldCovered((Id("b"), false)))
