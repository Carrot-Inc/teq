//> using platform js
//> using dep org.scala-js::scalajs-dom::2.8.1
// jars: scalajs-dom
// A jar's collection class that defines its iterator alone, scalajs-dom's `DOMListSeq` (snabbdom
// walks a node's children with it): `foreach` and what is built on it go through the iterator, as
// scala-library's `IterableOnceOps.foreach` does. A JS array is a `DOMList` to it.
import scala.scalajs.js
import org.scalajs.dom

@main def run(): Unit =
  val list = js.Array("a", "b", "c").asInstanceOf[dom.DOMList[String]]
  val seq = dom.DOMList.domListAsSeq(list)
  seq.foreach(println)
  println(seq.toList.map(_.toUpperCase))
  println(s"${seq.length} ${seq(1)} ${seq.mkString("-")}")
