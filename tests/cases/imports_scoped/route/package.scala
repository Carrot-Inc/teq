package app

import app.model.LoanModels.*
import ActionType.*

package object route:

  enum RouteError:
    case NotFound(path: String)
    case Forbidden

  trait Render[A]:
    def render(a: A): String

  given Render[RouteError] with
    def render(e: RouteError): String = e match
      case RouteError.NotFound(path) => s"404 $path"
      case RouteError.Forbidden => "403"

  val base: String = "/api/v1"

  def endpoint(name: String): String = s"$base/$name"

  def actionPath(a: ActionType): String = a match
    case Create => endpoint("create")
    case Cancel => endpoint("cancel")
    case Renew => endpoint("renew")
