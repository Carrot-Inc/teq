package meridian.web.client

import meridian.core.effect.*
import meridian.core.http.*

/** What a call to the backend yields: the value, or a reason the page can show. */
enum ApiError:
  case NoPermission(lacking: String)
  case NotFound
  case Invalid(message: String)
  case Unreachable(message: String)
  def describe: String = this match
    case NoPermission(l) => s"no permission: $l"
    case NotFound => "not found"
    case Invalid(m) => s"invalid: $m"
    case Unreachable(m) => s"unreachable: $m"

type ApiResult[A] = Either[ApiError, A]

/** The credentials a page holds and sends with every call. */
final case class Session(token: String, siteKey: Option[String], operator: String)

object ApiClient:
  def call[I, E, O](e: Endpoint[Credentials, I, E, O])(input: I): Eff[Transport & Session, Nothing, ApiResult[O]] =
    Eff.service[Session].flatMap { session =>
      Client.call(e)(Credentials(session.token, session.siteKey))(input).foldEff(
        {
          case ClientError.Status(401, _) => Eff.pure(Left(ApiError.NoPermission("token")))
          case ClientError.Status(403, body) => Eff.pure(Left(ApiError.NoPermission(body.getOrElse("?"))))
          case ClientError.Status(404, _) => Eff.pure(Left(ApiError.NotFound))
          case ClientError.Status(code, body) => Eff.pure(Left(ApiError.Invalid(s"$code ${body.getOrElse("")}")))
          case ClientError.Decode(message, _) => Eff.pure(Left(ApiError.Invalid(s"decode: $message")))
          case ClientError.Transport(message) => Eff.pure(Left(ApiError.Unreachable(message)))
        },
        {
          case Right(o) => Eff.pure(Right(o))
          case Left(err) => Eff.pure(Left(ApiError.Invalid(err.toString)))
        })
    }

  def callOption[I, E, O](e: Endpoint[Credentials, I, E, O])(input: I): Eff[Transport & Session, Nothing, ApiResult[Option[O]]] =
    call(e)(input).map {
      case Left(ApiError.NotFound) => Right(None)
      case Left(other) => Left(other)
      case Right(o) => Right(Some(o))
    }

  def foldResult[R, O](result: ApiResult[O], onError: ApiError => Eff[R, Nothing, Unit], onSuccess: O => Eff[R, Nothing, Unit]): Eff[R, Nothing, Unit] =
    result match
      case Left(e) => onError(e)
      case Right(o) => onSuccess(o)
