// Overloads in facades: every alternative of a member of a native JS type, of a native object and
// of a top-level native def is a way to call the one JavaScript function, so the alternatives
// keep the JavaScript name. Compiled together with tests/interop/scalajs-stub.
package overloads

import scala.scalajs.js
import scala.scalajs.js.annotation.{JSGlobal, JSImport, JSName}

@js.native @JSGlobal("Date")
class JsDate extends js.Object:
  def getTime(): Double = js.native
  def toISOString(): String = js.native

@js.native @JSGlobal("Math")
object JsMath extends js.Object:
  def max(a: Double, b: Double): Double = js.native
  def max(a: Double, b: Double, c: Double): Double = js.native
  def round(x: Double): Double = js.native

@js.native @JSGlobal("JSON")
object Json extends js.Object:
  def stringify(value: js.Any): String = js.native
  def stringify(value: js.Any, replacer: js.UndefOr[js.Any], indent: Int): String = js.native
  def stringify(value: js.Any, replacer: js.UndefOr[js.Any], indent: String): String = js.native

@js.native
trait Buffer extends js.Object:
  def slice(start: Int): Buffer = js.native
  def slice(start: Int, end: Int): Buffer = js.native
  def indexOf(value: String): Int = js.native
  def indexOf(value: Int): Int = js.native
  def indexOf(value: String, from: Int): Int = js.native
  @JSName("toString")
  def text(): String = js.native
  @JSName("toString")
  def text(encoding: String): String = js.native

@js.native @JSGlobal("Buffer")
object Buffer extends js.Object:
  def from(text: String): Buffer = js.native
  def from(text: String, encoding: String): Buffer = js.native
  def from(bytes: js.Array[Int]): Buffer = js.native

object Node:
  @js.native @JSImport("node:path", "join")
  def join(a: String, b: String): String = js.native
  @js.native @JSImport("node:path", "join")
  def join(a: String, b: String, c: String): String = js.native

@main def run(): Unit =
  println(JsMath.max(1, 2))
  println(JsMath.max(1, 5, 3))
  println(Json.stringify(js.Array(1, 2)))
  println(Json.stringify(js.Array(1, 2), js.undefined, 1).length)
  println(Json.stringify(js.Array(1), js.undefined, "--"))
  val b = Buffer.from("hello world")
  println(b.slice(6).text())
  println(b.slice(0, 5).text())
  println(b.indexOf("o"))
  println(b.indexOf("o", 5))
  println(b.indexOf(119))
  println(Buffer.from("aGk=", "base64").text())
  println(Buffer.from(js.Array(104, 105)).text("hex"))
  println(Node.join("a", "b"))
  println(Node.join("a", "b", "c"))
