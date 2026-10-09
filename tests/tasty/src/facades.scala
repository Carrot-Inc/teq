package fix.facades

import scala.scalajs.js
import scala.scalajs.js.annotation.*

@js.native @JSGlobal("Map")
class JsMap[K, V]() extends js.Object:
  def get(k: K): js.UndefOr[V] = js.native
  def set(k: K, v: V): this.type = js.native
  def has(k: K): Boolean = js.native
  def size: Int = js.native
  @JSName("delete") def remove(k: K): Boolean = js.native

@js.native @JSGlobal
class Date(ms: Double) extends js.Object:
  def getTime(): Double = js.native
  def toISOString(): String = js.native

@js.native @JSGlobal("Math")
object JsMath extends js.Object:
  def max(a: Double, b: Double): Double = js.native
  def max(a: Double, b: Double, c: Double): Double = js.native
  val PI: Double = js.native

@js.native @JSGlobalScope
object JsGlobals extends js.Object:
  def parseInt(s: String, radix: Int): Int = js.native
  val NaN: Double = js.native

@js.native
trait JsArr extends js.Object:
  @JSBracketAccess def apply(i: Int): String = js.native
  @JSBracketAccess def update(i: Int, v: String): Unit = js.native
  val length: Int = js.native
  @JSName("join") def joinWith(sep: String): String = js.native

@js.native @JSImport("node:path", JSImport.Namespace)
object JsPath extends js.Object:
  def basename(p: String): String = js.native
  val sep: String = js.native

@js.native @JSImport("node:path", JSImport.Default)
object JsPathDefault extends js.Object:
  def extname(p: String): String = js.native

@js.native @JSImport("node:events", "EventEmitter")
class JsEmitter() extends js.Object:
  def on(name: String, f: js.Function1[String, Unit]): this.type = js.native
  def emit(name: String, arg: String): Boolean = js.native
  def listenerCount(name: String): Int = js.native

object JsBindings:
  @js.native @JSImport("node:path", "join")
  def join(a: String, b: String): String = js.native
  @js.native @JSGlobal("encodeURIComponent")
  def encode(s: String): String = js.native

trait JsOptions extends js.Object:
  val name: String
  var verbose: js.UndefOr[Boolean] = js.undefined

class JsPoint(val x: Int, val y: Int) extends js.Object:
  def sum: Int = x + y

@js.native
trait JsMathOps extends js.Object:
  def abs(x: Double): Double = js.native
  def min(a: Double, b: Double): Double = js.native
  def min(a: Double, b: Double, c: Double): Double = js.native
  val E: Double = js.native

@js.native @JSGlobal("Math")
val math: JsMathOps = js.native
