// A pattern-matching anonymous function whose result type is a constructor variable that the
// expected type pins (`F` of `Kl { case .. }` against a `Kl[[x] =>> OptT[G, x], A, B]`, as
// http4s' routes are): the cases are typed against `OptT[G, B]`, so `OptT.none` finds its
// `Pure` for `G` and not an ambiguous one for an unknown constructor.
final case class Kl[F[_], -A, B](run: A => F[B])
final case class OptT[F[_], A](value: F[Option[A]])
object OptT:
  def none[F[_]: Pure, A]: OptT[F, A] = OptT(summon[Pure[F]].pure(None))
  def some[F[_]: Pure, A](a: A): OptT[F, A] = OptT(summon[Pure[F]].pure(Some(a)))
trait Pure[F[_]]:
  def pure[A](a: A): F[A]
given Pure[List] with
  def pure[A](a: A): List[A] = List(a)
given Pure[Vector] with
  def pure[A](a: A): Vector[A] = Vector(a)

final case class Req[F[_]](path: String)
type Http[F[_], G[_]] = Kl[F, Req[G], Int]
type Routes[F[_]] = Http[[x] =>> OptT[F, x], F]

def plain: Kl[[x] =>> OptT[List, x], String, Int] = Kl(s => OptT.none)
def cases: Kl[[x] =>> OptT[List, x], String, Int] = Kl {
  case s if s.nonEmpty => OptT.some(s.length)
  case _ => OptT.none
}
def routes(files: Routes[Vector]): Routes[Vector] = Kl {
  case r if r.path.startsWith("/api") => files.run(r)
  case _ => OptT.none
}

@main def main(): Unit =
  println(plain.run("a"))
  println(cases.run("abc"))
  println(cases.run(""))
  val files: Routes[Vector] = Kl(r => OptT.some(r.path.length))
  println(routes(files).run(Req("/api/x")))
  println(routes(files).run(Req("/")))
