package meridian.core.effect

enum Exit[+E, +A]:
  case Success(value: A)
  case Failure(cause: Cause[E])
  def toEither: Either[Cause[E], A] = this match
    case Success(a) => Right(a)
    case Failure(c) => Left(c)
  def getOrThrow(using ev: E <:< Throwable): A = this match
    case Success(a) => a
    case Failure(Cause.Fail(e)) => throw ev(e)
    case Failure(Cause.Die(t)) => throw t

enum Cause[+E]:
  case Fail(error: E)
  case Die(defect: Throwable)
  def prettyPrint: String = this match
    case Fail(e) => s"Fail($e)"
    case Die(t) => s"Die($t)"

/** Runs an effect to completion on the caller's stack, with an explicit continuation stack. */
object Runtime:
  private type Erased = Eff[Any, Any, Any]
  private type K = Any => Erased
  private enum Frame:
    case Continue(k: K)
    case Handle(onError: K, onSuccess: K)
    case Restore(env: Any)

  def run[R, E, A](eff: Eff[R, E, A], env: Env[R]): Exit[E, A] =
    var current: Erased = eff.asInstanceOf[Erased]
    var stack: List[Frame] = Nil
    var environment: Any = env
    var exit: Exit[Any, Any] | Null = null
    while exit == null do
      current match
        case Eff.Done(value) =>
          stack match
            case Frame.Continue(k) :: rest =>
              stack = rest
              current = k(value)
            case Frame.Handle(_, onSuccess) :: rest =>
              stack = rest
              current = onSuccess(value)
            case Frame.Restore(saved) :: rest =>
              stack = rest
              environment = saved
            case Nil => exit = Exit.Success(value)
        case Eff.Fail(error) =>
          stack match
            case Frame.Handle(onError, _) :: rest =>
              stack = rest
              current = onError(error)
            case Frame.Restore(saved) :: rest =>
              stack = rest
              environment = saved
            case _ :: rest => stack = rest
            case Nil => exit = Exit.Failure(Cause.Fail(error))
        case Eff.Suspend(thunk) =>
          current =
            try thunk().asInstanceOf[Erased]
            catch case defect: Throwable => Eff.Fail(defect)
        case Eff.Bind(source, next) =>
          stack = Frame.Continue(next.asInstanceOf[K]) :: stack
          current = source.asInstanceOf[Erased]
        case Eff.Fold(source, onError, onSuccess) =>
          stack = Frame.Handle(onError.asInstanceOf[K], onSuccess.asInstanceOf[K]) :: stack
          current = source.asInstanceOf[Erased]
        case Eff.Access(read) =>
          current =
            try Eff.Done(read(environment.asInstanceOf[Env[Any]]))
            catch case defect: Throwable => Eff.Fail(defect)
        case Eff.Provide(source, provided) =>
          stack = Frame.Restore(environment) :: stack
          environment = provided
          current = source.asInstanceOf[Erased]
    exit.asInstanceOf[Exit[E, A]]

  def unsafeRun[E, A](eff: Eff[Any, E, A]): Exit[E, A] = run(eff, Env.empty)

  def runOrThrow[A](eff: Eff[Any, Throwable, A]): A = unsafeRun(eff).getOrThrow

  def runEither[E, A](eff: Eff[Any, E, A]): Either[E, A] = unsafeRun(eff) match
    case Exit.Success(a) => Right(a)
    case Exit.Failure(Cause.Fail(e)) => Left(e)
    case Exit.Failure(Cause.Die(t)) => throw t

extension [A](task: Task[A])
  def runNow(): Unit =
    Runtime.unsafeRun(task) match
      case Exit.Failure(cause) => Log.error(s"unhandled: ${cause.prettyPrint}")
      case _ => ()
  def runSync(): A = Runtime.runOrThrow(task)

/** Lines written by the runtime and the applications, kept so that a self-test can print them. */
object Log:
  private var lines: List[String] = Nil
  def info(text: String): Unit = lines = text :: lines
  def error(text: String): Unit = lines = s"error: $text" :: lines
  def drain(): List[String] =
    val out = lines.reverse
    lines = Nil
    out
  def infoEff(text: => String): UIO[Unit] = Eff.succeed(info(text))
