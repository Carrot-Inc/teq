// jars: scala-library scalajs-dom
// scalajs-dom 2.8.1's facades from its jar, run under node, which has these globals without a
// browser: `console.log`, `fetch` over a `data:` URL with its `Response`, `Headers`, an
// `EventTarget` with a listener taking a `dom.Event`, `setTimeout`, and `BroadcastChannel` and
// `DOMException`, which the std's DOM layer does not define. With the jar on the class path every
// name is the jar's and the std's DOM files stay out; the expectation is written by hand, since
// there is no JVM build of scalajs-dom.
package domnode

import org.scalajs.dom
import scala.scalajs.js

@main def main(): Unit =
  dom.console.log("console", "one")
  val h = new dom.Headers()
  h.append("X-Kind", "probe")
  println(h.get("x-kind"))
  println(h.has("missing"))
  val target = new dom.EventTarget()
  target.addEventListener("ping", (e: dom.Event) => println(s"event ${e.`type`}"))
  target.dispatchEvent(new dom.Event("ping"))
  val channel = new dom.BroadcastChannel("probe")
  println(channel.name)
  channel.close()
  val ex = new dom.DOMException()
  println(ex.name)
  dom.fetch("data:text/plain,hello").`then`[Unit]((r: dom.Response) =>
    println(s"status ${r.status}")
    r.text().`then`[Unit]((t: String) => println(s"body $t")))
  js.timers.setTimeout(20)(println("timeout"))
