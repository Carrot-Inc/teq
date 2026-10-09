// A spliced tree cast to an abstract type member of a value's type keeps the cast's type, as
// zio's `Trace` macro builds its trace.
import scala.quoted.*

sealed trait Tracer:
  type Type <: AnyRef

object Tracer:
  val instance: Tracer = new Tracer { type Type = String }
  inline given auto: Tracer.instance.Type = ${ traceImpl }
  inline def explicit: Tracer.instance.Type = ${ traceImpl }
  def traceImpl(using Quotes): Expr[Tracer.instance.Type] =
    '{ ${ Expr("site") }.asInstanceOf[Tracer.instance.Type] }
