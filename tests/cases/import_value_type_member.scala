// A type member of a stable value, imported by name, renamed or through a wildcard, is the
// value's own `v.T`: a val of an object, a package's val and a local.
class Listener[E](val label: String):
  type Event = E
  type Handler = Event => String
  def fire(e: Event, h: Handler): String = s"$label ${h(e)}"

object dsl:
  val onClick: Listener[Int] = new Listener[Int]("click")

val onKey: Listener[Char] = new Listener[Char]("key")

import dsl.onClick.Event
import onKey.{Event as KeyEvent}

def next(e: Event): Event = e + 1
def upper(k: KeyEvent): KeyEvent = k.toUpper

@main def run(): Unit =
  println(dsl.onClick.fire(next(41), e => s"#$e"))
  println(onKey.fire(upper('q'), k => k.toString * 2))
  val local = new Listener[String]("text")
  import local.Event
  val s: Event = "x"
  locally:
    import local.*
    val h: Handler = t => t.reverse
    println(local.fire(s + "yz", h))
