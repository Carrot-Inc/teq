// An inner application under a by-name wrapper inside another application: `Some(a)`'s variable
// stays open through both instances for the cons's expected type to fix (`ConstraintHandling.approximation`).
package lambdaopenresultnestedapply

final case class Item[T](value: T)
def nested[A](a: A): List[Item[Option[A]]] =
  def delay[B](b: => B): List[B] = List(b)
  def wrap[B](b: B): B = b
  Item[Option[A]](None) :: wrap(delay(Item(Some(a))))
object Main:
  def main(args: Array[String]): Unit = println(nested("x").map(_.value))
