// An export's forwarder passes a by-name parameter on by name: evaluated where the member
// evaluates it, as often, never where the forwarder is called (Namer.addForwarder's call).
object Hooks:
  def lazily(body: => String): () => String = () => body
  def twice(body: => Int): Int = body + body

trait Prelude:
  export Hooks.{lazily, twice}

object P extends Prelude

object Q:
  export Hooks.twice

@main def run(): Unit =
  var n = 0
  val f = P.lazily { n += 1; s"call $n" }
  println(n)
  println(f())
  println(f())
  println(P.twice { n += 10; n })
  println(Q.twice { n += 100; n })
