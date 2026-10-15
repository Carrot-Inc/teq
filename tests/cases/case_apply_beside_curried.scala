import scala.annotation.targetName

// A case class's synthetic `apply(task, toMsg)` stays beside a companion's curried
// `apply(task)(toMessage)`, which matches it only flattened: dotty keeps the synthetic one unless a
// user `apply` matches it clause by clause (`Namer.invalidateIfClashingSynthetic`,
// `Type.matchesLoosely`), using clauses included, as tyrian's `Cmd.Run` relies on. A polymorphic
// method passed to the one alternative the arguments' shape leaves (`identity`) is typed against its
// parameter: an extension of the name in scope whose receiver cannot take the companion (the
// std's `Array.apply`) is no reason to type it ahead.
trait Applicative[F[_]]:
  def pure[A](a: A): F[A]
given Applicative[List] with
  def pure[A](a: A): List[A] = List(a)

final case class Run[F[_]: Applicative, A, Msg](task: F[A], toMsg: A => Msg):
  def map[OtherMsg](f: Msg => OtherMsg): Run[F, A, OtherMsg] = Run(task, toMsg andThen f)
object Run:
  @targetName("Run separate param lists")
  def apply[F[_]: Applicative, A, Msg](task: F[A])(toMessage: A => Msg): Run[F, A, Msg] =
    Run(task, toMessage)
  def apply[F[_]: Applicative, Msg](task: F[Msg]): Run[F, Msg, Msg] =
    Run(task, identity)

case class Plain[A, M](task: List[A], toMsg: A => M)
object Plain:
  def apply[M](task: List[M]): Plain[M, M] = Plain(task, identity)

def emit[F[_]: Applicative, Msg](msg: Msg): Run[F, Msg, Msg] =
  Run(summon[Applicative[F]].pure(msg), identity)

@main def run(): Unit =
  val r = Run(List(1), (i: Int) => "x" + i).map(_ + "!")
  println(r.toMsg(2) + " " + r.task)
  val c = Run(List(3))((i: Int) => i * 2)
  println(c.toMsg(4))
  val p = Run(List("p"))
  println(p.toMsg("q"))
  println(emit[List, String]("e").task)
  println(Plain(List(5)).toMsg(6))
