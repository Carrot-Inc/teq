package meridian.core.http

import meridian.core.effect.*

/** Sends requests somewhere: over the wire in an application, to a router in a self-test. */
trait Transport:
  def send(request: Request): Task[Response]

object Transport:
  given Tag[Transport] = Tag("Transport")
  def of(f: Request => Response): Transport = request => Eff.succeed(f(request))

/** A decoded reply: the endpoint's error type for an error status, its output otherwise. */
enum ClientError:
  case Transport(message: String)
  case Decode(message: String, response: Response)
  case Status(code: Int, body: Option[String])

object Client:
  def call[S, I, E, O](endpoint: Endpoint[S, I, E, O])(security: S)(input: I): Eff[Transport, ClientError, Either[E, O]] =
    val request = Wire.encode(endpoint.input, input, Wire.encode(endpoint.security, security, Request(endpoint.method, Nil, Nil, Nil, None)))
    Eff.service[Transport].flatMap { transport =>
      transport.send(request).mapError(t => ClientError.Transport(t.getMessage)).flatMap { response =>
        if response.status >= 200 && response.status < 300 then
          Eff.fromEither(Wire.decodeOut(endpoint.output, response).map(Right(_)).left.map(e => ClientError.Decode(e, response)))
        else
          Wire.decodeOut(endpoint.error, response) match
            case Right(e) => Eff.pure(Left(e))
            case Left(_) => Eff.fail(ClientError.Status(response.status, response.body))
      }
    }

  def callPlain[I, E, O](endpoint: Endpoint[Unit, I, E, O])(input: I): Eff[Transport, ClientError, Either[E, O]] =
    call(endpoint)(())(input)

  def render[S, I, E, O](endpoint: Endpoint[S, I, E, O])(security: S)(input: I): Request =
    Wire.encode(endpoint.input, input, Wire.encode(endpoint.security, security, Request(endpoint.method, Nil, Nil, Nil, None)))
