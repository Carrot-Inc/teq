package ia

abstract class Handler[A](val label: String):
  def handle(a: A): String
  def run(a: A): String = label + ": " + handle(a)

trait Logging:
  protected def log(msg: String): String = "[log] " + msg

class Counter:
  var count: Int = 0
  lazy val start: Int = 10
  def inc(): Unit = count += 1
