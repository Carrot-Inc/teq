import scala.collection.immutable.SortedSet

@main def run(): Unit =
  val a: SortedSet[Int] = SortedSet(3, 1) ++ SortedSet(2)
  println(a)
  val b: SortedSet[Int] = a ++ List(5, 4)
  println(b.toList)
  println((b -- List(1, 5)).toList)
