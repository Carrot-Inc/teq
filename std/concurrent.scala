// scala-library's `scala.concurrent` on JavaScript, as Scala.js's library has it: the global
// execution context runs each task as a microtask of the JS event loop, a callback always goes
// through its execution context (even on a completed future), and nothing blocks: `Await` on a
// completed future gives its value, on an incomplete one it waits no time and times out.
package scala.concurrent

import scala.concurrent.duration.Duration
import scala.util.{Failure, Success, Try}
import scala.util.control.NonFatal

type ExecutionException = java.util.concurrent.ExecutionException
type CancellationException = java.util.concurrent.CancellationException
type TimeoutException = java.util.concurrent.TimeoutException

def blocking[T](body: => T): T = body

trait ExecutionContext:
  def execute(runnable: Runnable): Unit
  def reportFailure(cause: Throwable): Unit
  def prepare(): ExecutionContext = this

trait ExecutionContextExecutor extends ExecutionContext, java.util.concurrent.Executor

@js("void Promise.resolve().then($0)")
def enqueueMicrotask(task: () => Unit): Unit

object ExecutionContext:
  lazy val global: ExecutionContextExecutor = new ExecutionContextExecutor:
    def execute(runnable: Runnable): Unit =
      enqueueMicrotask: () =>
        try runnable.run()
        catch case t: Throwable => reportFailure(t)
    def reportFailure(cause: Throwable): Unit = cause.printStackTrace()

  object Implicits:
    implicit def global: ExecutionContext = ExecutionContext.global

  object parasitic extends ExecutionContextExecutor:
    def execute(runnable: Runnable): Unit = runnable.run()
    def reportFailure(cause: Throwable): Unit = defaultReporter(cause)

  val defaultReporter: Throwable => Unit = _.printStackTrace()

  def fromExecutor(e: java.util.concurrent.Executor, reporter: Throwable => Unit): ExecutionContextExecutor =
    new ExecutionContextExecutor:
      def execute(runnable: Runnable): Unit = e.execute(runnable)
      def reportFailure(cause: Throwable): Unit = reporter(cause)
  def fromExecutor(e: java.util.concurrent.Executor): ExecutionContextExecutor = fromExecutor(e, defaultReporter)

sealed trait CanAwait
object AwaitPermission extends CanAwait

trait Awaitable[+T]:
  def ready(atMost: Duration)(implicit permit: CanAwait): this.type
  def result(atMost: Duration)(implicit permit: CanAwait): T

object Await:
  def ready[T](awaitable: Awaitable[T], atMost: Duration): awaitable.type = awaitable.ready(atMost)(AwaitPermission)
  def result[T](awaitable: Awaitable[T], atMost: Duration): T = awaitable.result(atMost)(AwaitPermission)

trait Future[+T] extends Awaitable[T]:
  def onComplete[U](f: Try[T] => U)(implicit executor: ExecutionContext): Unit
  def isCompleted: Boolean
  def value: Option[Try[T]]
  def transform[S](f: Try[T] => Try[S])(implicit executor: ExecutionContext): Future[S]
  def transformWith[S](f: Try[T] => Future[S])(implicit executor: ExecutionContext): Future[S]

  def failed: Future[Throwable] =
    transform(t =>
      t match
        case Failure(e) => Success(e)
        case Success(_) => Failure(new NoSuchElementException("Future.failed not completed with a throwable.")))(using ExecutionContext.parasitic)
  def foreach[U](f: T => U)(implicit executor: ExecutionContext): Unit = onComplete(_.foreach(f))
  def transform[S](s: T => S, f: Throwable => Throwable)(implicit executor: ExecutionContext): Future[S] =
    transform(t =>
      t match
        case Success(r) => Try(s(r))
        case Failure(e) => Try(throw f(e)))
  def map[S](f: T => S)(implicit executor: ExecutionContext): Future[S] = transform(_.map(f))
  def flatMap[S](f: T => Future[S])(implicit executor: ExecutionContext): Future[S] =
    transformWith(t =>
      t match
        case Success(r) => f(r)
        case Failure(_) => this.asInstanceOf[Future[S]])
  def flatten[S](implicit ev: T <:< Future[S]): Future[S] = flatMap(ev)(using ExecutionContext.parasitic)
  def filter(p: T => Boolean)(implicit executor: ExecutionContext): Future[T] =
    map(r => if p(r) then r else throw new NoSuchElementException("Future.filter predicate is not satisfied"))
  final def withFilter(p: T => Boolean)(implicit executor: ExecutionContext): Future[T] = filter(p)
  def collect[S](pf: PartialFunction[T, S])(implicit executor: ExecutionContext): Future[S] =
    map(r => if pf.isDefinedAt(r) then pf(r) else throw new NoSuchElementException("Future.collect partial function is not defined at: " + r))
  def recover[U >: T](pf: PartialFunction[Throwable, U])(implicit executor: ExecutionContext): Future[U] =
    transform(_.recover(pf))
  def recoverWith[U >: T](pf: PartialFunction[Throwable, Future[U]])(implicit executor: ExecutionContext): Future[U] =
    transformWith(t =>
      t match
        case Failure(e) if pf.isDefinedAt(e) => pf(e)
        case _ => this)
  def zip[U](that: Future[U]): Future[(T, U)] = zipWith(that)((a, b) => (a, b))(using ExecutionContext.parasitic)
  def zipWith[U, R](that: Future[U])(f: (T, U) => R)(implicit executor: ExecutionContext): Future[R] =
    flatMap(r1 => that.map(r2 => f(r1, r2)))(using ExecutionContext.parasitic)
  def fallbackTo[U >: T](that: Future[U]): Future[U] =
    if this eq that then this
    else
      transformWith(t =>
        t match
          case Success(_) => this
          case Failure(_) => that.transform(tt => if tt.isSuccess then tt else t)(using ExecutionContext.parasitic))(using ExecutionContext.parasitic)
  def andThen[U](pf: PartialFunction[Try[T], U])(implicit executor: ExecutionContext): Future[T] =
    transform(result =>
      try if pf.isDefinedAt(result) then pf(result)
      catch case t if NonFatal(t) => executor.reportFailure(t)
      result)

object Future:
  val unit: Future[Unit] = successful(())
  final def never: Future[Nothing] = new impl.DefaultPromise[Nothing]()
  def successful[T](result: T): Future[T] = Promise.successful(result).future
  def failed[T](exception: Throwable): Future[T] = Promise.failed(exception).future
  def fromTry[T](result: Try[T]): Future[T] = Promise.fromTry(result).future
  def apply[T](body: => T)(implicit executor: ExecutionContext): Future[T] = unit.map(_ => body)
  def delegate[T](body: => Future[T])(implicit executor: ExecutionContext): Future[T] = unit.flatMap(_ => body)

  def sequence[A, CC[X] <: IterableOnce[X], To](in: CC[Future[A]])(implicit bf: scala.collection.BuildFrom[CC[Future[A]], A, To], executor: ExecutionContext): Future[To] =
    var acc: Future[scala.collection.mutable.Builder[A, To]] = successful(bf.newBuilder(in))
    in.iterator.foreach(fa => acc = acc.zipWith(fa)((b, a) => b.addOne(a)))
    acc.map(_.result())(using ExecutionContext.parasitic)

  def traverse[A, B, M[X] <: IterableOnce[X]](in: M[A])(fn: A => Future[B])(implicit bf: scala.collection.BuildFrom[M[A], B, M[B]], executor: ExecutionContext): Future[M[B]] =
    var acc: Future[scala.collection.mutable.Builder[B, M[B]]] = successful(bf.newBuilder(in))
    in.iterator.foreach(a => acc = acc.zipWith(fn(a))((b, x) => b.addOne(x)))
    acc.map(_.result())(using ExecutionContext.parasitic)

  def firstCompletedOf[T](futures: IterableOnce[Future[T]])(implicit executor: ExecutionContext): Future[T] =
    val p = Promise[T]()
    futures.iterator.foreach(f => f.onComplete(t => p.tryComplete(t)))
    p.future

  def foldLeft[T, R](futures: scala.collection.immutable.Iterable[Future[T]])(zero: R)(op: (R, T) => R)(implicit executor: ExecutionContext): Future[R] =
    var acc: Future[R] = successful(zero)
    futures.foreach(f => acc = acc.zipWith(f)(op))
    acc

trait Promise[T]:
  def future: Future[T]
  def isCompleted: Boolean
  def tryComplete(result: Try[T]): Boolean
  def complete(result: Try[T]): this.type =
    if tryComplete(result) then this else throw new IllegalStateException("Promise already completed.")
  def completeWith(other: Future[T]): this.type =
    if other ne future then other.onComplete(t => tryComplete(t))(using ExecutionContext.parasitic)
    this
  def success(value: T): this.type = complete(Success(value))
  def trySuccess(value: T): Boolean = tryComplete(Success(value))
  def failure(cause: Throwable): this.type = complete(Failure(cause))
  def tryFailure(cause: Throwable): Boolean = tryComplete(Failure(cause))

object Promise:
  def apply[T](): Promise[T] = new impl.DefaultPromise[T]()
  def failed[T](exception: Throwable): Promise[T] = fromTry(Failure(exception))
  def successful[T](result: T): Promise[T] = fromTry(Success(result))
  def fromTry[T](result: Try[T]): Promise[T] =
    val p = new impl.DefaultPromise[T]()
    p.tryComplete(result)
    p

package impl:
  // The promise and its future in one object, as scala-library's: the callbacks registered
  // before completion wait in a list and are handed to their execution contexts on completion.
  final class DefaultPromise[T] extends Promise[T], Future[T]:
    private var outcome: Try[T] | Null = null
    private var waiting: Array[Try[T] => Unit] = Array.empty
    def future: Future[T] = this
    def isCompleted: Boolean = outcome != null
    def value: Option[Try[T]] = if outcome == null then None else Some(outcome.asInstanceOf[Try[T]])
    def tryComplete(result: Try[T]): Boolean =
      if outcome != null then false
      else
        outcome = result
        val pending = waiting
        waiting = Array.empty
        var i = 0
        while i < pending.length do
          pending(i)(result)
          i += 1
        true
    private def dispatch(k: Try[T] => Unit): Unit =
      if outcome != null then k(outcome.asInstanceOf[Try[T]]) else waiting.push(k)
    // A callback's context is prepared when it is registered, as scala-library's
    // `Transformation` does.
    def onComplete[U](f: Try[T] => U)(implicit executor: ExecutionContext): Unit =
      val ec = executor.prepare()
      dispatch(t => ec.execute(() => f(t)))
    def transform[S](f: Try[T] => Try[S])(implicit executor: ExecutionContext): Future[S] =
      val p = new DefaultPromise[S]()
      val ec = executor.prepare()
      dispatch(t =>
        ec.execute(() =>
          val next =
            try f(t)
            catch case e if NonFatal(e) => Failure(e)
          p.tryComplete(next)))
      p
    def transformWith[S](f: Try[T] => Future[S])(implicit executor: ExecutionContext): Future[S] =
      val p = new DefaultPromise[S]()
      val ec = executor.prepare()
      dispatch(t =>
        ec.execute(() =>
          try p.completeWith(f(t))
          catch case e if NonFatal(e) => p.tryComplete(Failure(e))))
      p
    def ready(atMost: Duration)(implicit permit: CanAwait): this.type =
      if outcome != null then this
      else if atMost eq Duration.Undefined then throw new IllegalArgumentException("Cannot wait for Undefined duration of time")
      else throw new java.util.concurrent.TimeoutException("Future timed out after [" + atMost + "]")
    def result(atMost: Duration)(implicit permit: CanAwait): T = ready(atMost).value.get.get
    override def toString: String =
      if outcome == null then "Future(<not completed>)" else "Future(" + outcome + ")"
