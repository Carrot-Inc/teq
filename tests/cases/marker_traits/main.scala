import lib.{Failure, Product2Like, Serializable, Throwable}

enum ApiError extends Throwable:
  case NotFound
  case Forbidden
  case Invalid(field: String, reason: String)
  case Wrapped(cause: Throwable)

  override def getCause: Option[Throwable] = this match
    case Wrapped(cause) => Some(cause)
    case _ => None

enum HttpError(val status: Int) extends Throwable, Serializable:
  case BadRequest extends HttpError(400)
  case Server(detail: String) extends HttpError(500)

  override def getMessage: String = s"HTTP $status"

final case class Timeout(millis: Int) extends Throwable, Serializable, Product2Like[Int, String]

final case class Attempt(label: String, error: Throwable, previous: Option[Throwable] = None)

def describe(t: Throwable): String = t match
  case ApiError.NotFound => "not found"
  case ApiError.Invalid(field, reason) => s"$field is invalid: $reason"
  case ApiError.Wrapped(inner) => "wrapped(" + describe(inner) + ")"
  case e: ApiError => "api error " + e.ordinal
  case h: HttpError => "http " + h.status
  case Timeout(ms) => s"timeout after $ms ms"
  case other => "other: " + other.getMessage

def run(n: Int): Either[Throwable, Int] =
  if n < 0 then Left(ApiError.Invalid("n", "negative"))
  else if n == 0 then Left(HttpError.BadRequest)
  else if n > 100 then Left(Failure("too large"))
  else Right(n * 2)

def rootCause(t: Throwable): Throwable = t.getCause match
  case Some(c) => rootCause(c)
  case None => t

@main def main(): Unit =
  val errors: List[Throwable] = List(
    ApiError.NotFound,
    ApiError.Forbidden,
    ApiError.Invalid("email", "empty"),
    ApiError.Wrapped(HttpError.Server("db down")),
    HttpError.BadRequest,
    Timeout(30),
    Failure("boom"),
    Failure.apply2("outer", ApiError.Wrapped(Timeout(5))),
  )
  errors.foreach(e => println(describe(e)))
  errors.foreach(e => println(e.getMessage))
  println(errors.map(rootCause))
  println(List(-1, 0, 7, 500).map(run))
  val attempt = Attempt("first", ApiError.Forbidden)
  println(attempt)
  println(attempt.copy(previous = Some(Timeout(1))))
  println(attempt.error == ApiError.Forbidden)
  println(attempt == Attempt("first", ApiError.Forbidden))
  val serializable: List[Serializable] = List(HttpError.BadRequest, Timeout(2), HttpError.Server("x"))
  println(serializable)
  println(HttpError.Server("x").status)
