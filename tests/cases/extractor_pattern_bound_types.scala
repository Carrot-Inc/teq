// An extractor whose type arguments the scrutinee leaves undetermined: `maximizeType` makes a pattern-bound type for
// each (`Bind.unapply[Any, E$1 @ E$1, A$2 @ A$2, B$1 @ B$1]` in scalac's tree), which its products bind in the pattern.
sealed trait Step[-R, +E, +A]
object Step:
  final case class Done[A](value: A) extends Step[Any, Nothing, A]
  final case class Bind[R, E, A, B](source: Step[R, E, A], next: A => Step[R, E, B]) extends Step[R, E, B]

object Run:
  private type Erased = Step[Any, Any, Any]
  private type K = Any => Erased

  def run(step: Step[Any, Any, Any]): Any =
    var current: Erased = step
    var stack: List[K] = Nil
    var result: Any = null
    while result == null do
      current match
        case Step.Done(value) =>
          stack match
            case k :: rest =>
              stack = rest
              current = k(value)
            case Nil => result = value
        case b @ Step.Bind(source, next) =>
          stack = next.asInstanceOf[K] :: stack
          current = source.asInstanceOf[Erased]
    result

// A binder whose bound names a later one: `A$1 <: Iterable[B$1]`.
final case class Holder[A <: Iterable[B], B](value: A)

def size(x: Any): Int = x match
  case Holder(v) => v.size
  case _ => -1

@main def main(): Unit =
  val program: Step[Any, Any, Any] = Step.Bind[Any, Nothing, Int, Int](Step.Done(20), (n: Int) => Step.Bind[Any, Nothing, Int, Int](Step.Done(n + 1), (m: Int) => Step.Done(m * 2)))
  println(Run.run(program))
  println(Run.run(Step.Done("done")))
  println(size(Holder(List(1, 2, 3))))
  println(size("no"))
