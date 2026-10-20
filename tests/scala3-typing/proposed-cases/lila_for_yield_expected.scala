// lila writes `def system(...): Funit = for _ <- persistLine(...) yield publish(...)` where the
// yield's value is not Unit (a Future, a kamon Counter...): the for's `map` lambda is typed
// against `map`'s result type B, bounded by Unit through the expected `Funit`, and scalac
// discards the value. The same lambda written out as `.map(_ => ...)` is accepted by teq.
import scala.concurrent.{Await, Future, ExecutionContext}
import scala.concurrent.duration.*
import ExecutionContext.Implicits.global

type Funit = Future[Unit]
var published = 0
def persistLine(): Funit = Future.successful(())
def publish(): Int = { published += 1; published }

def system(): Funit =
  for _ <- persistLine()
  yield publish()

def written(): Funit = persistLine().map(_ => publish())

def widened: Option[Long] = for x <- Option(1) yield x + 1

@main def run() =
  Await.result(system(), 5.seconds)
  Await.result(written(), 5.seconds)
  println(s"published $published, widened $widened")
