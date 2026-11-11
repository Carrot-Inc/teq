// An alias applied past its own parameters whose expansion is a type lambda (`Tuple.IsMappedBy[F][T]`'s shape): the
// application is read through its constructor's supertype, so its implicit scope, its class and a `null` test see the
// type it stands for.
type Curried[F[_]] = [X] =>> X =:= X

@main def main(): Unit =
  println(summon[Curried[Option][Int]](1))
  val ev = summon[Tuple.IsMappedBy[Option][(Option[Int], Option[String])]]
  println(ev((Some(1), None)))
  val c: Curried[Option][Int] = summon[Int =:= Int]
  println(c != null)
