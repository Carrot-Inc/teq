// A by-name argument whose type variable the expected result bounds by `Unit` has its value
// discarded, as dotc adapts to a type that derives from `Unit`: `eff[Unit] { migrate() }` for a
// `Task[Unit]`, evaluated when the task runs.
final class Task[+A](val run: () => A)
def eff[A](a: => A): Task[A] = Task(() => a)
object Flyway:
  var migrated = 0
  def migrate(): String = { migrated += 1; "migrated" }
  def clean(): Int = { migrated -= 1; migrated }

trait Controls:
  def migrate: Task[Unit]
  def clean: Task[Unit]

def controls: Controls = new Controls:
  override def migrate: Task[Unit] =
    eff { Flyway.migrate() }
  override def clean: Task[Unit] =
    eff { Flyway.clean() }

@main def run(): Unit =
  val c = controls
  val m = c.migrate
  println(Flyway.migrated)
  m.run()
  m.run()
  println(Flyway.migrated)
  c.clean.run()
  println(Flyway.migrated)
