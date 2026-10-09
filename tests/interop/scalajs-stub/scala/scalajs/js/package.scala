// A minimal scala.scalajs.js for the interop tests, written in the facade syntax that teq
// understands natively. The real library lives outside this repository.
package scala.scalajs.js

@js.native
trait Any

@js.native
@JSGlobal("Object")
class Object extends Any

@js.native
@JSGlobal("Object")
object Object extends Object:
  def assign(target: Object, sources: Object*): Object = js.native
  def keys(o: Object): Array[String] = js.native

def undefined: Unit = ()
def native: Nothing = ???
def defined[A](a: A): UndefOr[A] = a

@js("(typeof $0)")
def typeOf(x: scala.Any): String

@js("($0 === undefined)")
def isUndefined(x: scala.Any): Boolean

type UndefOr[+A] = A | Unit
type Function0[+R] = () => R
type Function1[-A, +R] = A => R
type Function2[-A, -B, +R] = (A, B) => R
type Function3[-A, -B, -C, +R] = (A, B, C) => R

@js.native
@JSGlobal("Array")
class Array[A] extends Object:
  def length: Int = js.native
  def push(items: A*): Int = js.native
  def map[B](f: Function1[A, B]): Array[B] = js.native
  def join(separator: String = js.native): String = js.native
  @JSBracketAccess
  def apply(index: Int): A = js.native
  @JSBracketAccess
  def update(index: Int, value: A): Unit = js.native

object Array:
  @js("[...$1]")
  def apply[A](items: A*): Array[A]

@js.native
trait Dictionary[A] extends Object:
  @JSBracketAccess
  def apply(key: String): A = js.native
  @JSBracketAccess
  def update(key: String, value: A): Unit = js.native

object Dictionary:
  @js("$jsObj(...$1)")
  def apply[A](fields: (String, A)*): Dictionary[A]

@js.native
@JSGlobal("Promise")
class Promise[+A](executor: Function2[Function1[A, Unit], Function1[scala.Any, Unit], Unit]) extends Object:
  def `then`[B](onFulfilled: Function1[A, B]): Promise[B] = js.native

@js.native
@JSGlobal("Promise")
object Promise extends Object:
  def resolve[A](value: A): Promise[A] = js.native

@js.native
@JSGlobal("Date")
class Date(value: Double = js.native) extends Object:
  def getTime(): Double = js.native
  def getFullYear(): Int = js.native
  def getHours(): Int = js.native
  def toISOString(): String = js.native

@js.native
@JSGlobal("Date")
object Date extends Object:
  def now(): Double = js.native
  def UTC(year: Int, month: Int, date: Int = js.native, hours: Int = js.native): Double = js.native

@js.native
@JSGlobal("JSON")
object JSON extends Object:
  def parse(text: String): Dynamic = js.native
  def stringify(value: scala.Any): String = js.native

@js.native
@JSGlobal("Math")
object Math extends Object:
  def random(): Double = js.native
  def max(values: Double*): Double = js.native

@js.native
trait Dynamic extends Any

object Dynamic:
  @js.native
  @JSGlobalScope
  object global extends Dynamic

  @js("$jsObj(...$1)")
  def literal(fields: (String, scala.Any)*): Dynamic

@js.native
trait Tuple2[+A, +B] extends Object

object Tuple2:
  @js("[$1, $2]")
  def apply[A, B](_1: A, _2: B): Tuple2[A, B]

extension [A, B](t: Tuple2[A, B])
  @js("$0[0]")
  def _1: A
  @js("$0[1]")
  def _2: B

object JSConverters:
  extension [A](o: Option[A])
    def orUndefined: UndefOr[A] = o match
      case Some(a) => a
      case None => undefined
  extension [A](xs: Seq[A])
    def toJSArray: Array[A] = Array(xs*)
  extension [A](m: Map[String, A])
    def toJSDictionary: Dictionary[A] = Dictionary(m.toList*)
  // The names Scala.js gives its implicit classes, importable one by one.
  given JSRichOption: JSConverters.type = JSConverters
  given JSRichMap: JSConverters.type = JSConverters
  given JSRichIterableOnce: JSConverters.type = JSConverters
