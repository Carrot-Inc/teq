// A class for each trait, which writes the trait's accessors.
class OfWritten extends Written
class OfArgument extends Argument
class OfOperands extends Operands
class OfDemanded extends Demanded
class OfDropped extends Dropped
class OfMoved extends Moved
class OfGetterAfter extends GetterAfter
class OfGetterRight extends GetterRight
class OfGetterLeft extends GetterLeft
class OfSummed extends Summed

@main def run(): Unit =
  val values = List(new OfWritten().c(), new OfArgument().n(0, 0), new OfOperands().sum, new OfDemanded().one, new OfDropped().out,
    new OfMoved().out, new OfGetterAfter().out, new OfGetterRight().out, new OfGetterLeft().out)
  println(values.mkString(" ") + " " + new OfOperands().text + " " + new OfSummed().sum(new Pt(2)).x)
