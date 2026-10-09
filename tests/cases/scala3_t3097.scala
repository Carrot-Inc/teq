// Adapted from scala3 tests/run/t3097.scala (Apache-2.0, see tests/scala3/README.md); replaced: extends App.
sealed trait ISimpleValue

sealed trait IListValue extends ISimpleValue
sealed trait IAtomicValue[O] extends ISimpleValue

sealed trait IAbstractDoubleValue[O] extends IAtomicValue[O]
sealed trait IDoubleValue extends IAbstractDoubleValue[Double]

case class ListValue(val items: List[IAtomicValue[_]]) extends IListValue
class DoubleValue(val data: Double) extends IDoubleValue

object Test {
  def main(args: Array[String]): Unit = ()
  // match is exhaustive
  (new DoubleValue(1): ISimpleValue) match {
    case m: IListValue => println("list")
    case a: IAtomicValue[_] => println("atomic")
  }
}
