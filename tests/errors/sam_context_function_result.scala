// A trait whose single method returns a context function type is no SAM type for a function
// literal, as scalac's implementation restriction has it (scala3's neg/i4611b); an anonymous
// class implements it.
// expect: Implementation restriction: cannot convert this expression to `Responder[Response]` because its result type
class Response
class Request
object Request:
  type To[T] = Request ?=> T

trait Responder[T]:
  def responseFor(value: T): Request.To[Response]

object Responder:
  val lambda: Responder[Response] = response => response
  val anon: Responder[Response] = new Responder[Response]:
    def responseFor(value: Response): Request.To[Response] = value
