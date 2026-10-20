// A lambda argument whose result type variable R is bounded above only by a type that still holds
// another open type variable: Future[S] of an enclosing flatMap (lila: bindForm / fold inside
// `.flatMap:`), or Future[A] of `NoBot[A <: Result]`. scalac adapts the body to R (given
// Conversion, value discarding); teq types the body with no expected type.
import scala.language.implicitConversions
import scala.concurrent.{Await, Future, ExecutionContext}
import scala.concurrent.duration.*
import ExecutionContext.Implicits.global

class Result(val s: String)
given Conversion[Result, Future[Result]] = Future.successful(_)
def bindForm[R](ok: Boolean)(error: String => R, success: Int => R): R =
  if ok then success(1) else error("bad")
def jsonFormError(e: String): Result = Result(s"error $e")

def check(ok: Boolean): Future[Result] =
  Future.successful(ok).flatMap(b => bindForm(b)(jsonFormError, i => Future.successful(Result(s"ok $i"))))

def NoBot[A <: Result](a: => Future[A]): Future[Result] = a
def vote(ok: Boolean): Future[Result] =
  NoBot(bindForm(ok)(jsonFormError, i => Future.successful(Result(s"voted $i"))))

var recorded = 0
def record(n: Int): Int = { recorded += n; recorded }
def ensure(): Future[Unit] = Future.successful(1).flatMap(_ => Future.successful(2).map(record(_)))

@main def run() =
  def get[A](f: Future[A]) = Await.result(f, 5.seconds)
  println(List(get(check(true)), get(check(false)), get(vote(false))).map(_.s).mkString(", "))
  get(ensure())
  println(s"recorded $recorded")
