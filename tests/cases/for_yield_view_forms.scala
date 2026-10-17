// The forms of a lambda's result adapted to an open variable through the expected type's view, for
// a for-comprehension's synthesized lambda as for a written one: an overloaded `map`, a returned
// lambda, nested generators, a yield that is itself a lambda (its parameter's type from the bound),
// an eta-expanded method, and a variable bounded by a type that holds another open variable, which
// the search approximates by a wildcard of that variable's bounds (`ProtoTypes.wildApprox`).
package foryieldviewforms

import scala.language.implicitConversions

final class Task[A](val run: () => A):
  def map[B](f: A => B): Task[B] = Task(() => f(run()))
  def map[B](f: A => B, label: String): Task[B] = Task(() => f(run()))
  def flatMap[B](f: A => Task[B]): Task[B] = Task(() => f(run()).run())
object Task:
  def succeed[A](a: => A): Task[A] = Task(() => a)

trait ToRaw { def raw: Raw }
final case class Raw(name: String, value: String)
object ToRaw:
  implicit def rawToRaw(r: Raw): ToRaw = new ToRaw { def raw = r }
implicit def rawToList(r: Raw): List[String] = List(r.name, r.value)

def overloaded: Task[ToRaw] = for stamp <- Task.succeed(1L) yield Raw("Overloaded", stamp.toString)
def returned: Long => Task[ToRaw] = n => for stamp <- Task.succeed(n) yield Raw("Returned", stamp.toString)
def nested: Task[ToRaw] = for
  a <- Task.succeed(1L)
  b <- Task.succeed(2L)
  c <- Task.succeed(3L)
yield Raw("Nested", s"$a$b$c")
def lambdaYield: Task[Long => ToRaw] = for n <- Task.succeed(1L) yield (m => Raw("Lambda", (n + m).toString))
def makeRaw(n: Long): Raw = Raw("Eta", n.toString)
def eta: Task[ToRaw] = Task.succeed(1L).map(makeRaw)
def wrap[X](t: Task[List[X]]): List[X] = t.run()

@main def main(): Unit =
  println(overloaded.run().raw.name)
  println(returned(7L).run().raw.value)
  println(nested.run().raw.value)
  println(lambdaYield.run()(2L).raw.value)
  println(eta.run().raw.name)
  println(wrap(for n <- Task.succeed(1L) yield Raw("a", n.toString)))
  println(wrap(Task.succeed(2L).map(n => Raw("b", n.toString))))
