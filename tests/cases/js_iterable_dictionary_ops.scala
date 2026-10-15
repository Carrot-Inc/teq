//> using platform js
//> using dep org.scala-js::scalajs-dom::2.8.1
// jars: scalajs-dom
import scala.scalajs.js
import scala.scalajs.js.typedarray.Int8Array
import org.scalajs.dom

// Scala.js's conversions of `js.Any`'s companion, which teq's companions of the JavaScript types
// hold: a dictionary is a map whose updates write through to it (tyrian's `dataset.collect`), an
// iterable a Scala `Iterable` (a typed array read as an `IterableOnce`, `dom.Headers` mapped past
// its private constructor parameter `map`).
@main def run(): Unit =
  val dict = js.Dictionary("tyrianFlagA" -> "1", "other" -> "2", "tyrianFlagB" -> "3")
  val flags = dict.collect { case (k, v) if k.startsWith("tyrianFlag") => k.replaceFirst("^tyrianFlag", "") -> v }.toMap
  println(flags.toList.sorted)
  dict += ("added" -> "4")
  println(dict("added"))
  val buf = new Int8Array(3)
  buf(0) = 1
  buf(1) = -2
  buf(2) = 3
  val bytes: IArray[Byte] = IArray.from(buf)
  println(bytes.toList)
  val it: js.Iterable[Int] = js.Array(4, 5)
  println(it.map(_ * 2).toList)
  val headers = new dom.Headers()
  headers.append("Content-Type", "text/plain")
  headers.append("X-Count", "2")
  println(headers.map(_.toList.mkString("=")).toList.sorted)
