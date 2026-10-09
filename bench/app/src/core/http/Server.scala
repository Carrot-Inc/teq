package meridian.core.http

import meridian.core.effect.*

/** An endpoint with the effect that serves it. */
final class Route[-R](val method: Method, val template: String, val handle: Request => Eff[R, Nothing, Option[Response]]):
  def tag: String = s"$method $template"

object Route:
  def of[R, S, I, E, O](endpoint: Endpoint[S, I, E, O])(logic: S => I => Eff[R, E, O]): Route[R] =
    new Route[R](endpoint.method, endpoint.path, request =>
      if request.method != endpoint.method then Eff.pure(None)
      else
        Wire.decode(endpoint.security, request, 0) match
          case Left(_) => Eff.pure(Some(Response(401, Nil, Some("\"unauthorised\""))))
          case Right((security, _)) =>
            Wire.decode(endpoint.input, request, 0) match
              case Left(error) => Eff.pure(if error.startsWith("expected segment") || error.startsWith("missing segment") then None else Some(Response(400, Nil, Some("\"" + error + "\""))))
              case Right((input, consumed)) =>
                if consumed != request.segments.length then Eff.pure(None)
                else
                  logic(security)(input).foldEff(
                    e => Eff.pure(Some(Wire.encodeOut(endpoint.error, e, Response(422, Nil, None)))),
                    o => Eff.pure(Some(Wire.encodeOut(endpoint.output, o, Response(200, Nil, None))))))

  def plain[R, I, E, O](endpoint: Endpoint[Unit, I, E, O])(logic: I => Eff[R, E, O]): Route[R] = of(endpoint)(_ => logic)

final class Routes[-R](val routes: List[Route[R]]):
  def ++[R1 <: R](that: Routes[R1]): Routes[R1] = Routes(routes ++ that.routes)
  def handle(request: Request): Eff[R, Nothing, Response] =
    Eff.loop[R, Nothing, (List[Route[R]], Option[Response])]((routes, None))(
      state => state._1.nonEmpty && state._2.isEmpty,
      state => state._1.head.handle(request).map(found => (state._1.tail, found)))
      .map(_._2.getOrElse(Response(404, Nil, Some("\"not found\""))))
  def transport(env: Env[R]): Transport = request => Eff.succeed(Runtime.run(handle(request), env) match
    case Exit.Success(response) => response
    case Exit.Failure(cause) => Response(500, Nil, Some("\"" + cause.prettyPrint + "\"")))
  def describe: List[String] = routes.map(_.tag)

object Routes:
  def apply[R](routes: List[Route[R]]): Routes[R] = new Routes(routes)
  def of[R](routes: Route[R]*): Routes[R] = new Routes(routes.toList)
