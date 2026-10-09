// Scala.js's `js.Thenable`, the shape `js.Promise` extends, and the conversion of one to a
// `Future`: fulfilled completes it with the value, rejected fails it with the reason, a
// `Throwable` as itself and any other value wrapped in a `JavaScriptException`.
package scala.scalajs.js

import scala.concurrent.Future

@js.native
trait Thenable[+A] extends Object:
  def `then`[B](onFulfilled: Function1[A, B | Thenable[B]], onRejected: UndefOr[Function1[scala.Any, B | Thenable[B]]] = js.native): Thenable[B] = js.native

object Thenable:
  object ThenableConversions:
    extension [A](p: Thenable[A])
      def toFuture: Future[A] =
        val done = scala.concurrent.Promise[A]()
        p.`then`[Unit](
          (v: A) => done.success(v): Unit,
          (e: scala.Any) =>
            done.failure(e match
              case th: Throwable => th
              case _ => JavaScriptException(e)
            ): Unit
        )
        done.future
  given ThenableOps: ThenableConversions.type = ThenableConversions

  object Implicits:
    implicit def thenable2future[A](p: Thenable[A]): Future[A] = ThenableConversions.toFuture(p)
