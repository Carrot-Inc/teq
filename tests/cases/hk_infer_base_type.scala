// A higher-kinded type variable applied to fewer arguments than the class of the argument has
// is the constructor of a base type of that class, the first in its linearisation that fits the
// variable's bound, as dotty's canInstantiate lifts the argument (liftToBase): a Map as M[A] with
// M[A] <: IterableOnce[A] is an Iterable of pairs, which an untupled lambda takes.
import scala.collection.BuildFrom

extension [A, M[A] <: IterableOnce[A]](xs: M[A])
  def parallel[B](f: A => Option[B])(using bf: BuildFrom[M[A], B, M[B]]): Option[M[B]] =
    Some(bf.fromSpecific(xs)(xs.iterator.flatMap(f)))

@main def run(): Unit =
  val perfs: Map[String, Int] = Map("a" -> 1, "b" -> 2)
  println(perfs.parallel: (k, v) =>
    Some(s"$k=$v"))
  println(List(1, 2, 3).parallel(x => Option.when(x > 1)(x * 10)))
  println(Vector("x").parallel(s => Some(s.length)))
