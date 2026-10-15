//> using platform js
// Typed arrays made from a `js.Array` of their elements, as Scala.js's constructors take a `js.Iterable`,
// and their companions' `BYTES_PER_ELEMENT` (indigo's renderer), and views of a buffer whose offset and
// length are left out, JavaScript's defaults (sttp's `FetchBackend`).
import scala.scalajs.js
import scala.scalajs.js.typedarray.*

@main def run(): Unit =
  val data = js.Array(1.5f, 2.5f)
  val f = new Float32Array(data)
  println(s"${f.length} ${f(1)} ${Float32Array.BYTES_PER_ELEMENT} ${Int16Array.BYTES_PER_ELEMENT}")
  val u = new Uint16Array(js.Array(1, 2, 3))
  println(s"${u.length} ${u(2)}")
  val buf = new ArrayBuffer(8)
  val all = new Int8Array(buf)
  val tail = new Int16Array(buf, 2)
  println(s"${all.length} ${tail.length}")
