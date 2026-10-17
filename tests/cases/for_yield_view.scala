// A for-comprehension's synthesized lambdas typed as written ones: the yield's body under `map`'s
// open result `B`, which the declared result bounds, adapted to it through the expected type's
// implicit view (`Typer.adapt`'s `adaptToSubType`, the view searched against the variable's bounds
// as `Implicits.viewProto` and `ProtoTypes.wildApprox` approximate them); one conversion per run.
package forview

import scala.language.implicitConversions

final class Task[A](val run: () => A):
  def map[B](f: A => B): Task[B] = Task(() => f(run()))
  def flatMap[B](f: A => Task[B]): Task[B] = Task(() => f(run()).run())
object Task:
  def succeed[A](a: => A): Task[A] = Task(() => a)

trait ToRaw { def raw: Raw }
final case class Raw(name: String, value: String)
object ToRaw:
  var conversions = 0
  implicit def rawToRaw(r: Raw): ToRaw =
    conversions += 1
    new ToRaw { def raw = r }

def sign(url: String): Task[ToRaw] = for
  id    <- Task.succeed(url.hashCode)
  stamp <- Task.succeed(42L)
yield Raw("Authorization", s"$id:$stamp")

def take(t: Task[ToRaw]): String = t.run().raw.name
def takeByName(t: => Task[ToRaw]): String = t.run().raw.name

@main def main(): Unit =
  println(sign("x").run().raw.name)
  var assigned: Task[ToRaw] = Task.succeed(Raw("none", "").asInstanceOf[ToRaw])
  assigned = for stamp <- Task.succeed(1L) yield Raw("Assigned", stamp.toString)
  println(assigned.run().raw.name)
  println(take(for stamp <- Task.succeed(2L) yield Raw("Argument", stamp.toString)))
  println(takeByName(for stamp <- Task.succeed(3L) yield Raw("ByName", stamp.toString)))
  val block: Task[ToRaw] = for stamp <- Task.succeed(4L) yield { val r = Raw("Block", stamp.toString); r }
  println(block.run().raw.name)
  val plain: Task[ToRaw] = Task.succeed(5L).map(stamp => Raw("Plain", stamp.toString))
  println(plain.run().raw.name)
  println(ToRaw.conversions)
