// lila's UserLogins.distinctRecent: `map.view.map(Dated.apply)` on a Map, whose `map` is
// overloaded (`map[B](f: ((K, V)) => B)` and `map[K2, V2](f: ((K, V)) => (K2, V2))`). All
// alternatives take a unary function of the same pair, so scalac pre-types the argument
// against `((K, V)) => ?` (Applications.pretypeArgs) and eta-expands the two-parameter
// `Dated.apply` into a function of the pair. teq rejects the method reference.
case class Dated[V](value: V, date: Long)

def distinctRecent[V](all: List[Dated[V]]): Iterable[Dated[V]] =
  all
    .foldLeft(Map.empty[V, Long]): (acc, d) =>
      if acc.contains(d.value) then acc else acc + (d.value -> d.date)
    .view
    .map(Dated.apply)

@main def run() =
  println(distinctRecent(List(Dated("a", 3L), Dated("b", 2L), Dated("a", 1L))).toList)
