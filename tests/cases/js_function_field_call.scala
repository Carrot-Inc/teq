//> using platform js
// A JS function a field holds, called: it gets no receiver, as Scala.js calls a function value,
// where `(o.f)(x)` would pass `o` (cats-effect's `BatchingMacrotaskExecutor` keeps the browser's
// `queueMicrotask` in a field, which refuses any other receiver).
import scala.scalajs.js

def receiverTest: js.Function0[String] =
  js.Dynamic.newInstance(js.Dynamic.global.Function)("return this === undefined || this === globalThis ? 'no receiver' : 'a receiver'").asInstanceOf[js.Function0[String]]

class Holder:
  val f: js.Function0[String] = receiverTest

object Kept:
  val g: js.Function0[String] = receiverTest

@main def run(): Unit =
  val h = new Holder
  println(h.f())
  println(Kept.g())
  val local = h.f
  println(local())
