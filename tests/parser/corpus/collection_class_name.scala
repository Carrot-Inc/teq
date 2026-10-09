// A collection's `collectionClassName`, the name its `toString` gives it, which a library reads
// through a reflective call (munit prints a collection so); `private[scala]` to a program.
import scala.collection.mutable
import scala.reflect.Selectable.reflectiveSelectable

@main def run =
  val xs: List[Iterable[Any]] = List(List(1), Vector(1), Seq(1), IndexedSeq(1), Set(1), Map(1 -> 2),
    mutable.ArrayBuffer(1), mutable.ListBuffer(1), mutable.HashMap(1 -> 2), mutable.Set(1), 1 to 3,
    LazyList(1), Array(1).toSeq, Nil, Map(1 -> 2).keySet, List(1).view, Set(1, 2, 3, 4, 5),
    Map(1 -> 1, 2 -> 2, 3 -> 3, 4 -> 4, 5 -> 5))
  xs.foreach(x => println(reflectiveSelectable(x).selectDynamic("collectionClassName")))
