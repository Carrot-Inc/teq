// The slice of scala.scalajs.js the app uses, written in the facade syntax teq understands natively.
// Every teq value already is a JS value, so js.Any is Scala's Any; the native classes below describe the
// JavaScript builtins, and nothing is emitted for them.
package scala.scalajs.js

import scala.scalajs.js

type Any = scala.Any

object Any:
  // The conversions a `_sjs1_` jar's bodies call by name: a JS array's collection methods and the
  // array as a sequence, a dictionary as a map whose updates write through to it (scala-java-time's
  // tzdb provider reads its zone tables so).
  def jsArrayOps[A](array: Array[A]): ArrayOps[A] = new ArrayOps(array)
  def wrapArray[A](array: Array[A]): WrappedArray[A] = new WrappedArray(array)
  def wrapDictionary[A](dict: Dictionary[A]): WrappedDictionary[A] = new WrappedDictionary(dict)
  def fromString(s: String): Any = s
  def fromInt(i: Int): Any = i
  def fromDouble(d: Double): Any = d
  def fromBoolean(b: Boolean): Any = b
  def fromUnit(u: Unit): Any = u
  def fromFunction0[R](f: () => R): Function0[R] = f
  def fromFunction1[A, R](f: A => R): Function1[A, R] = f
  def fromFunction2[A, B, R](f: (A, B) => R): Function2[A, B, R] = f
  def fromFunction3[A, B, C, R](f: (A, B, C) => R): Function3[A, B, C, R] = f

  /** The operations of `js.Object`'s companion that are no JavaScript methods of it. */
  implicit final class ObjectCompanionOps(private val self: Object.type):
    def hasProperty(o: Object, p: String): Boolean = special.in(p, o)
    def properties(o: scala.Any): Array[String] = propertyNames(o)

@js("(() => { const names = []; for (const name in $0) names.push(name); return names; })()")
private[js] def propertyNames(o: scala.Any): Array[String]

@js.native
@JSGlobal("Object")
class Object:
  def hasOwnProperty(v: String): Boolean = js.native
  def propertyIsEnumerable(v: String): Boolean = js.native
  def isPrototypeOf(v: Object): Boolean = js.native
  def toLocaleString(): String = js.native
  def valueOf(): scala.Any = js.native

@js.native
@JSGlobal("Object")
object Object extends Object:
  def assign(target: Object, sources: Object*): Object = js.native
  def keys(o: Object): Array[String] = js.native
  def values(o: Object): Array[scala.Any] = js.native
  def entries(o: Object): Array[Tuple2[String, scala.Any]] = js.native
  def freeze[T <: Object](o: T): T = js.native
  def isFrozen(o: Object): Boolean = js.native
  def getPrototypeOf(o: Object): Object = js.native
  def is(a: scala.Any, b: scala.Any): Boolean = js.native
  def hasOwn(o: Object, key: String): Boolean = js.native
  def getOwnPropertyNames(o: Object): Array[String] = js.native
  def defineProperty(o: Object, p: String, attributes: PropertyDescriptor): o.type = js.native
  def getOwnPropertyDescriptor(o: Object, p: String): UndefOr[PropertyDescriptor] = js.native
  def create(o: Object, properties: scala.Any): Object = js.native
  def create(o: Object): Object = js.native

trait PropertyDescriptor extends Object:
  var configurable: UndefOr[Boolean] = js.undefined
  var enumerable: UndefOr[Boolean] = js.undefined
  var value: UndefOr[scala.Any] = js.undefined
  var writable: UndefOr[Boolean] = js.undefined
  var get: UndefOr[Function0[scala.Any]] = js.undefined
  var set: UndefOr[Function1[scala.Any, scala.Any]] = js.undefined

/** A JavaScript symbol, a primitive value. */
@js.native
trait Symbol extends scala.Any

object Symbol:
  @js("Symbol()")
  def apply(): Symbol
  @js("Symbol($1)")
  def apply(description: String): Symbol
  @js("Symbol.for($1)")
  def forKey(key: String): Symbol
  @js("Symbol.keyFor($1)")
  def keyFor(sym: Symbol): UndefOr[String]
  @js("Symbol.iterator")
  def iterator: Symbol
  @js("Symbol.asyncIterator")
  def asyncIterator: Symbol

def undefined: Unit = ()

/** The JavaScript class value of `T`, a native JavaScript class, which the compiler gives. */
inline def constructorOf[T <: scala.Any]: Dynamic = compiletime.error("js.constructorOf")

/** The body of a native member; the compiler drops it, so it is never evaluated. */
def native: Nothing = ???

/** An object, as in Scala.js, whose `apply` a jar's body names (`js.defined.apply[A](a)`). */
object defined:
  def apply[A](a: A): UndefOr[A] = a

@js("(typeof $0)")
def typeOf(x: scala.Any): String

@js("($0 === undefined)")
def isUndefined(x: scala.Any): Boolean

@js("eval($0)")
def eval(code: String): Dynamic

@js("new $DictMap($0)")
private[js] def dictionaryEntries[A](dict: Dictionary[A]): scala.RawMap[String, A]

type UndefOr[+A] = A | Unit

/** Scala.js's pseudo-union `js.|`, which a program imports to write `A | B` before Scala 3 had unions: Scala
  * 3's union, as `UndefOr` is. */
type |[A, B] = A | B

/** Scala.js's dynamic import: `body` is evaluated in a later task, as Scala.js links it into a module the
  * promise's `import()` loads, which teq's one file holds already; its result fulfils the promise, what it throws
  * rejects it. */
def dynamicImport[A](body: => A): Promise[A] =
  new Promise[A]((resolve: Function1[A | Thenable[A], scala.Any], reject: Function1[scala.Any, scala.Any]) =>
    timers.setTimeout(0) {
      try resolve(body)
      catch case th: Throwable => reject(special.unwrapFromThrowable(th))
    }: Unit)

/** A JavaScript function. teq's functions are JavaScript functions, and each conforms to it. */
@js.native
@JSGlobal("Function")
class Function(args: String*) extends Object:
  val length: Int = js.native
  def call(thisArg: scala.Any, argArray: scala.Any*): Dynamic = js.native
  def bind(thisArg: scala.Any, argArray: scala.Any*): Dynamic = js.native

@js.native
@JSGlobal("Function")
object Function extends Object:
  def apply(args: String*): Function = js.native

/** A JavaScript function that receives its `this` as the first argument of the Scala function it
  * is made from: a `function`, where a Scala function is an arrow function.
  */
@js.native
trait ThisFunction extends Function

object ThisFunction:
  @js("((f) => function(...a) { return f(this, ...a); })($1)")
  implicit def fromFunction1[T1, R](f: scala.Function1[T1, R]): ThisFunction0[T1, R]
  @js("((f) => function(...a) { return f(this, ...a); })($1)")
  implicit def fromFunction2[T1, T2, R](f: scala.Function2[T1, T2, R]): ThisFunction1[T1, T2, R]
  @js("((f) => function(...a) { return f(this, ...a); })($1)")
  implicit def fromFunction3[T1, T2, T3, R](f: scala.Function3[T1, T2, T3, R]): ThisFunction2[T1, T2, T3, R]
  @js("((f) => function(...a) { return f(this, ...a); })($1)")
  implicit def fromFunction4[T1, T2, T3, T4, R](f: scala.Function4[T1, T2, T3, T4, R]): ThisFunction3[T1, T2, T3, T4, R]
  @js("((f) => function(...a) { return f(this, ...a); })($1)")
  implicit def fromFunction5[T1, T2, T3, T4, T5, R](f: scala.Function5[T1, T2, T3, T4, T5, R]): ThisFunction4[T1, T2, T3, T4, T5, R]
  @js("((f) => function(...a) { return f(this, ...a); })($1)")
  implicit def fromFunction6[T1, T2, T3, T4, T5, T6, R](f: scala.Function6[T1, T2, T3, T4, T5, T6, R]): ThisFunction5[T1, T2, T3, T4, T5, T6, R]
  @js("((f) => function(...a) { return f(this, ...a); })($1)")
  implicit def fromFunction7[T1, T2, T3, T4, T5, T6, T7, R](f: scala.Function7[T1, T2, T3, T4, T5, T6, T7, R]): ThisFunction6[T1, T2, T3, T4, T5, T6, T7, R]
  @js("((f) => (...a) => f.call(...a))($1)")
  implicit def toFunction1[T1, R](f: ThisFunction0[T1, R]): scala.Function1[T1, R]
  @js("((f) => (...a) => f.call(...a))($1)")
  implicit def toFunction2[T1, T2, R](f: ThisFunction1[T1, T2, R]): scala.Function2[T1, T2, R]
  @js("((f) => (...a) => f.call(...a))($1)")
  implicit def toFunction3[T1, T2, T3, R](f: ThisFunction2[T1, T2, T3, R]): scala.Function3[T1, T2, T3, R]
  @js("((f) => (...a) => f.call(...a))($1)")
  implicit def toFunction4[T1, T2, T3, T4, R](f: ThisFunction3[T1, T2, T3, T4, R]): scala.Function4[T1, T2, T3, T4, R]
  @js("((f) => (...a) => f.call(...a))($1)")
  implicit def toFunction5[T1, T2, T3, T4, T5, R](f: ThisFunction4[T1, T2, T3, T4, T5, R]): scala.Function5[T1, T2, T3, T4, T5, R]
  @js("((f) => (...a) => f.call(...a))($1)")
  implicit def toFunction6[T1, T2, T3, T4, T5, T6, R](f: ThisFunction5[T1, T2, T3, T4, T5, T6, R]): scala.Function6[T1, T2, T3, T4, T5, T6, R]
  @js("((f) => (...a) => f.call(...a))($1)")
  implicit def toFunction7[T1, T2, T3, T4, T5, T6, T7, R](f: ThisFunction6[T1, T2, T3, T4, T5, T6, T7, R]): scala.Function7[T1, T2, T3, T4, T5, T6, T7, R]

@js.native
trait ThisFunction0[-T0, +R] extends ThisFunction:
  @js("$0.call($1)")
  def apply(thisArg: T0): R

@js.native
trait ThisFunction1[-T0, -T1, +R] extends ThisFunction:
  @js("$0.call($1, $2)")
  def apply(thisArg: T0, arg1: T1): R

@js.native
trait ThisFunction2[-T0, -T1, -T2, +R] extends ThisFunction:
  @js("$0.call($1, $2, $3)")
  def apply(thisArg: T0, arg1: T1, arg2: T2): R

@js.native
trait ThisFunction3[-T0, -T1, -T2, -T3, +R] extends ThisFunction:
  @js("$0.call($1, $2, $3, $4)")
  def apply(thisArg: T0, arg1: T1, arg2: T2, arg3: T3): R

@js.native
trait ThisFunction4[-T0, -T1, -T2, -T3, -T4, +R] extends ThisFunction:
  @js("$0.call($1, $2, $3, $4, $5)")
  def apply(thisArg: T0, arg1: T1, arg2: T2, arg3: T3, arg4: T4): R

@js.native
trait ThisFunction5[-T0, -T1, -T2, -T3, -T4, -T5, +R] extends ThisFunction:
  @js("$0.call($1, $2, $3, $4, $5, $6)")
  def apply(thisArg: T0, arg1: T1, arg2: T2, arg3: T3, arg4: T4, arg5: T5): R

@js.native
trait ThisFunction6[-T0, -T1, -T2, -T3, -T4, -T5, -T6, +R] extends ThisFunction:
  @js("$0.call($1, $2, $3, $4, $5, $6, $7)")
  def apply(thisArg: T0, arg1: T1, arg2: T2, arg3: T3, arg4: T4, arg5: T5, arg6: T6): R

type Function0[+R] = () => R
type Function1[-A, +R] = A => R
type Function2[-A, -B, +R] = (A, B) => R
type Function3[-A, -B, -C, +R] = (A, B, C) => R
type Function4[-A, -B, -C, -D, +R] = (A, B, C, D) => R
type Function5[-A, -B, -C, -D, -E, +R] = (A, B, C, D, E) => R
type Function6[-A, -B, -C, -D, -E, -F, +R] = (A, B, C, D, E, F) => R

/** Scala.js's `js.Array`, the JavaScript array: a type of its own beside Scala's `Array` (one
  * JavaScript value in teq), whose collection methods are `js.ArrayOps`'s, building `js.Array`s
  * without a `ClassTag`. */
@js.native
@JSGlobal
class Array[A](arrayLength: Int = js.native) extends Object, Iterable[A]:
  def length: Int = js.native
  @js("$0.length = $1")
  def length_=(n: Int): Unit
  @JSBracketAccess
  def apply(index: Int): A = js.native
  @JSBracketAccess
  def update(index: Int, value: A): Unit = js.native
  def concat[B >: A](items: Array[? <: B]*): Array[B] = js.native
  def join(seperator: String = js.native): String = js.native
  def pop(): A = js.native
  def push(items: A*): Int = js.native
  @JSName("reverse")
  def reverseInPlace(): Array[A] = js.native
  def shift(): A = js.native
  @JSName("slice")
  def jsSlice(start: Int = 0, end: Int = Int.MaxValue): Array[A] = js.native
  @js("$0.sort()")
  def sort(): Array[A]
  def sort(compareFn: Function2[A, A, Int]): Array[A] = js.native
  def splice(index: Int, deleteCount: Int, items: A*): Array[A] = js.native
  def unshift(items: A*): Int = js.native
  def indexOf(searchElement: A): Int = js.native
  def lastIndexOf(searchElement: A): Int = js.native

object Array extends LowPrioArrayConversions:
  @js("[...$1]")
  def apply[A](items: A*): Array[A]
  @js("Array.isArray($1)")
  def isArray(arg: scala.Any): Boolean
  @js("Array.from($1)")
  def from[A](iterable: Iterable[A]): Array[A]
  // Scala.js's `js.Any.jsArrayOps`, found in the implicit scope of `js.Any`, which the array's
  // companion stands for here, `js.Any` being `scala.Any`: `arr.map(f)` is a `js.Array`.
  implicit def jsArrayOps[A](array: Array[A]): ArrayOps[A] = new ArrayOps(array)

// Scala.js's `js.LowPrioAnyImplicits.wrapArray`: the array as a sequence where one is expected,
// below `jsArrayOps` for the methods both have.
trait LowPrioArrayConversions:
  implicit def wrapArray[A](array: Array[A]): WrappedArray[A] = new WrappedArray(array)

/** Scala.js's `js.ArrayOps`: the collection methods of a `js.Array`, each building a `js.Array`. */
final class ArrayOps[A](private val array: Array[A]) extends scala.IndexedSeqOps[A, Array, Array[A]]:
  def length: Int = array.length
  def apply(i: Int): A =
    if i < 0 || i >= array.length then throw new IndexOutOfBoundsException(i.toString) else array(i)
  def buildCC[B](items: scala.RawBuffer[B]): Array[B] = scala.unsafeCast[scala.RawBuffer[B], Array[B]](items)
  override def iterator: scala.Iterator[A] = scala.Iterator.over(scala.unsafeCast[Array[A], scala.RawBuffer[A]](array))
  def foreach[U](f: A => U): Unit =
    var i = 0
    while i < array.length do
      f(array(i))
      i += 1
  override def className: String = "ArrayOps"
  // Scala.js's in-place operations, each the array itself after it. No member here is overloaded
  // (`remove(index, count)` is left out): a JVM build, where this layer is open too, names every
  // overloaded member's alternatives, which would complete this class over the lean std's traits.
  def +=(elem: A): Array[A] =
    array.push(elem)
    array
  def addOne(elem: A): Array[A] = this += elem
  def append(elem: A): Array[A] = this += elem
  def ++=(elems: IterableOnce[A]): Array[A] =
    elems.iterator.foreach(x => array.push(x))
    array
  def addAll(elems: IterableOnce[A]): Array[A] = this ++= elems
  def appendAll(elems: IterableOnce[A]): Array[A] = this ++= elems
  def +=:(elem: A): Array[A] =
    array.unshift(elem)
    array
  def prepend(elem: A): Array[A] = elem +=: this
  def ++=:(elems: IterableOnce[A]): Array[A] =
    val items = elems.iterator.toList
    items.reverseIterator.foreach(x => array.unshift(x))
    array
  def prependAll(elems: IterableOnce[A]): Array[A] = elems ++=: this
  def insert(index: Int, elem: A): Unit =
    if index < 0 || index > array.length then throw new IndexOutOfBoundsException(index.toString)
    array.splice(index, 0, elem)
  def insertAll(index: Int, elems: IterableOnce[A]): Unit =
    if index < 0 || index > array.length then throw new IndexOutOfBoundsException(index.toString)
    var at = index
    elems.iterator.foreach { x =>
      array.splice(at, 0, x)
      at += 1
    }
  def remove(index: Int): A =
    if index < 0 || index >= array.length then throw new IndexOutOfBoundsException(index.toString)
    array.splice(index, 1)(0)
  def -=(elem: A): Array[A] =
    val i = indexOf(elem)
    if i >= 0 then array.splice(i, 1)
    array
  def subtractOne(elem: A): Array[A] = this -= elem
  def clear(): Unit = array.length = 0
  def trimStart(n: Int): Unit = array.splice(0, math.min(math.max(n, 0), array.length))
  def trimEnd(n: Int): Unit = array.length = math.max(0, array.length - math.max(n, 0))
  def filterInPlace(p: A => Boolean): Array[A] =
    var i = 0
    while i < array.length do
      if p(array(i)) then i += 1 else array.splice(i, 1)
    array
  def mapInPlace(f: A => A): Array[A] =
    var i = 0
    while i < array.length do
      array(i) = f(array(i))
      i += 1
    array

/** Scala.js's `js.WrappedArray`: a `js.Array` as a mutable indexed sequence, its updates the
  * array's. */
final class WrappedArray[A](private val array: Array[A]) extends scala.SeqOps[A, WrappedArray, WrappedArray[A]], scala.IndexedSeq[A]:
  def this() = this(Array[A]())
  def length: Int = array.length
  def apply(i: Int): A =
    if i < 0 || i >= array.length then throw new IndexOutOfBoundsException(i.toString) else array(i)
  def update(i: Int, elem: A): Unit =
    if i < 0 || i >= array.length then throw new IndexOutOfBoundsException(i.toString) else array(i) = elem
  def addOne(elem: A): this.type =
    array.push(elem)
    this
  def +=(elem: A): this.type = addOne(elem)
  def buildCC[B](items: scala.RawBuffer[B]): WrappedArray[B] = new WrappedArray(scala.unsafeCast[scala.RawBuffer[B], Array[B]](items))
  override def iterator: scala.Iterator[A] = scala.Iterator.over(scala.unsafeCast[Array[A], scala.RawBuffer[A]](array))
  override def className: String = "WrappedArray"

@js.native
trait Dictionary[A] extends Any:
  @JSBracketAccess
  def apply(key: String): A = js.native
  @JSBracketAccess
  def update(key: String, value: A): Unit = js.native

object Dictionary:
  // Scala.js's `js.Any.wrapDictionary`, found in the implicit scope of `js.Any`, which a
  // dictionary's companion stands for here, `js.Any` being `scala.Any`: a map whose updates write
  // through to the dictionary (`dataset.collect { .. }`).
  implicit def wrapDictionary[A](dict: Dictionary[A]): WrappedDictionary[A] = Any.wrapDictionary(dict)
  @js("$jsObj(...$1)")
  def apply[A](fields: (String, A)*): Dictionary[A]
  @js("({})")
  def empty[A]: Dictionary[A]

/** Scala.js's `js.WrappedDictionary`: a dictionary as a mutable map whose updates write through to it, its `map` of
  * pairs one again, and the dictionary again by its companion's `toJSDictionary` (`dict.addAll(..)` where a
  * dictionary is expected). */
final class WrappedDictionary[A](private val dict: Dictionary[A]) extends scala.collection.mutable.HashMap[String, A](dictionaryEntries(dict)):
  def map[B](f: ((String, A)) => (String, B)): WrappedDictionary[B] =
    val out = Dictionary.empty[B]
    foreach { e =>
      val (k, v) = f(e)
      out(k) = v
    }
    new WrappedDictionary(out)

object WrappedDictionary:
  def empty[A]: WrappedDictionary[A] = new WrappedDictionary(Dictionary.empty[A])
  implicit def toJSDictionary[A](wrappedDict: WrappedDictionary[A]): Dictionary[A] = wrappedDict.dict

@js.native
trait Dynamic

object Dynamic:
  /** Members are bare global identifiers, so a bundler's `define` can replace them. */
  @js.native
  @JSGlobalScope
  object global extends Dynamic

  // Scala.js's `literal` is an object applied dynamically: a jar's bodies call
  // `literal.applyDynamic("apply")(pairs)`, a program `literal("a" -> 1)`.
  object literal:
    @js("$jsObj(...$1)")
    def apply(fields: (String, scala.Any)*): Object & Dynamic
    @js("$jsObj(...$2)")
    def applyDynamic(name: String)(fields: (String, scala.Any)*): Object & Dynamic
    @js("$jsObj(...$2)")
    def applyDynamicNamed(name: String)(fields: (String, scala.Any)*): Object & Dynamic

  @js("new ($1)(...$2)")
  def newInstance(clazz: Dynamic)(args: scala.Any*): Object & Dynamic

/** A two-element JS array; teq tuples are objects, so the conversion is explicit. */
/** The iteration protocol: `for..of` over a value with a `Symbol.iterator` method. */
@js.native
trait Iterable[+A] extends Object:
  @js("$0[Symbol.iterator]()")
  def jsIterator(): Iterator[A]

object Iterable:
  // Scala.js's `js.Any.iterableOps`, found in the implicit scope of `js.Any`, which an iterable's
  // companion stands for here: the JavaScript iterable as a Scala one, iterated afresh by each
  // `iterator` (a `dom.Headers` mapped, a typed array read as an `IterableOnce`).
  implicit def iterableOps[A](iterable: Iterable[A]): scala.Iterable[A] =
    new scala.collection.AbstractIterable[A]:
      def buildCC[B](items: scala.RawBuffer[B]): scala.Iterable[B] = scala.fromArray(items)
      override def iterator: scala.Iterator[A] = Iterator.JsIteratorOps.toIterator(iterable.jsIterator())
      def foreach[U](f: A => U): Unit = iterator.foreach(f)

@js.native
trait Iterator[+A] extends Object:
  def next(): Iterator.Entry[A] = js.native

object Iterator:
  @js.native
  trait Entry[+A] extends Object:
    def done: Boolean = js.native
    def value: A = js.native
  object JsIteratorOps:
    extension [A](self: Iterator[A])
      def toIterator: scala.Iterator[A] = new scala.Iterator[A]:
        private var entry = self.next()
        def hasNext: Boolean = !entry.done
        def next(): A =
          val v = entry.value
          entry = self.next()
          v
  given IteratorOps: JsIteratorOps.type = JsIteratorOps

@js.native
trait Tuple2[+A, +B] extends Object:
  @annotation.JSName("0") val _1: A = js.native
  @annotation.JSName("1") val _2: B = js.native

object Tuple2:
  @js("[$1, $2]")
  def apply[A, B](_1: A, _2: B): Tuple2[A, B]
  def unapply[A, B](t: Tuple2[A, B]): Some[(A, B)] = Some((t._1, t._2))
  implicit def fromScalaTuple2[A, B](t: (A, B)): Tuple2[A, B] = apply(t._1, t._2)
  implicit def toScalaTuple2[A, B](t: Tuple2[A, B]): (A, B) = (t._1, t._2)

@js.native
trait Tuple3[+A, +B, +C] extends Object:
  @annotation.JSName("0") val _1: A = js.native
  @annotation.JSName("1") val _2: B = js.native
  @annotation.JSName("2") val _3: C = js.native

object Tuple3:
  @js("[$1, $2, $3]")
  def apply[A, B, C](_1: A, _2: B, _3: C): Tuple3[A, B, C]
  def unapply[A, B, C](t: Tuple3[A, B, C]): Some[(A, B, C)] = Some((t._1, t._2, t._3))
  implicit def fromScalaTuple3[A, B, C](t: (A, B, C)): Tuple3[A, B, C] = apply(t._1, t._2, t._3)
  implicit def toScalaTuple3[A, B, C](t: Tuple3[A, B, C]): (A, B, C) = (t._1, t._2, t._3)

@js.native
@JSGlobal("Promise")
class Promise[+A](executor: Function2[Function1[A | Thenable[A], ?], Function1[scala.Any, ?], ?]) extends Object, Thenable[A]:
  override def `then`[B](onFulfilled: Function1[A, B | Thenable[B]], onRejected: UndefOr[Function1[scala.Any, B | Thenable[B]]] = js.native): Promise[B] = js.native
  def `catch`[B >: A](onRejected: Function1[scala.Any, B | Promise[B]]): Promise[B] = js.native
  def `finally`(onFinally: Function0[scala.Any]): Promise[A] = js.native

@js.native
@JSGlobal("Promise")
object Promise extends Object:
  def resolve[A](value: A): Promise[A] = js.native
  def reject(reason: scala.Any): Promise[Nothing] = js.native
  def all[A](promises: Array[Promise[A]]): Promise[Array[A]] = js.native
  def race[A](promises: Array[Promise[A]]): Promise[A] = js.native
  def allSettled[A](promises: Array[Promise[A]]): Promise[Array[Object]] = js.native

/** `new Date()` is now, `new Date(millis)` or `new Date(text)` one value, `new Date(year, month, ...)` local fields. */
@js.native
@JSGlobal("Date")
class Date(
  value: Double | Int | String = js.native,
  month: Int = js.native,
  date: Int = js.native,
  hours: Int = js.native,
  minutes: Int = js.native,
  seconds: Int = js.native,
  ms: Int = js.native,
) extends Object:
  def getTime(): Double = js.native
  def valueOf(): Double = js.native
  def getFullYear(): Int = js.native
  def getMonth(): Int = js.native
  def getDate(): Int = js.native
  def getDay(): Int = js.native
  def getHours(): Int = js.native
  def getMinutes(): Int = js.native
  def getSeconds(): Int = js.native
  def getMilliseconds(): Int = js.native
  def getTimezoneOffset(): Int = js.native
  def getUTCFullYear(): Int = js.native
  def getUTCMonth(): Int = js.native
  def getUTCDate(): Int = js.native
  def getUTCDay(): Int = js.native
  def getUTCHours(): Int = js.native
  def getUTCMinutes(): Int = js.native
  def getUTCSeconds(): Int = js.native
  def getUTCMilliseconds(): Int = js.native
  def setTime(time: Double): Double = js.native
  def setFullYear(year: Int, month: Int = js.native, date: Int = js.native): Double = js.native
  def setMonth(month: Int, date: Int = js.native): Double = js.native
  def setDate(date: Int): Double = js.native
  def setHours(hours: Int, minutes: Int = js.native, seconds: Int = js.native, ms: Int = js.native): Double = js.native
  def setMinutes(minutes: Int, seconds: Int = js.native, ms: Int = js.native): Double = js.native
  def setSeconds(seconds: Int, ms: Int = js.native): Double = js.native
  def setMilliseconds(ms: Int): Double = js.native
  def toISOString(): String = js.native
  def toJSON(): String = js.native
  def toDateString(): String = js.native
  def toTimeString(): String = js.native
  def toUTCString(): String = js.native
  def toLocaleDateString(locales: String = js.native, options: Object = js.native): String = js.native
  def toLocaleTimeString(locales: String = js.native, options: Object = js.native): String = js.native
  def toLocaleString(locales: String = js.native, options: Object = js.native): String = js.native

@js.native
@JSGlobal("Date")
object Date extends Object:
  def now(): Double = js.native
  def parse(text: String): Double = js.native
  def UTC(year: Int, month: Int, date: Int = js.native, hours: Int = js.native, minutes: Int = js.native, seconds: Int = js.native, ms: Int = js.native): Double = js.native

@js.native
@JSGlobal("JSON")
object JSON extends Object:
  def parse(text: String, reviver: Function2[scala.Any, scala.Any, scala.Any] = js.native): Dynamic = js.native
  def stringify(value: scala.Any, replacer: scala.Any = js.native, space: scala.Any = js.native): String = js.native

@js.native
@JSGlobal("Math")
object Math extends Object:
  val E: Double = js.native
  val PI: Double = js.native
  def abs(x: Double): Double = js.native
  def ceil(x: Double): Double = js.native
  def floor(x: Double): Double = js.native
  def round(x: Double): Double = js.native
  def trunc(x: Double): Double = js.native
  def sign(x: Double): Double = js.native
  def sqrt(x: Double): Double = js.native
  def cbrt(x: Double): Double = js.native
  def pow(x: Double, y: Double): Double = js.native
  def exp(x: Double): Double = js.native
  def log(x: Double): Double = js.native
  def log2(x: Double): Double = js.native
  def log10(x: Double): Double = js.native
  def sin(x: Double): Double = js.native
  def cos(x: Double): Double = js.native
  def tan(x: Double): Double = js.native
  def asin(x: Double): Double = js.native
  def acos(x: Double): Double = js.native
  def atan(x: Double): Double = js.native
  def atan2(y: Double, x: Double): Double = js.native
  def hypot(values: Double*): Double = js.native
  def max(value1: Int, values: Int*): Int = js.native
  def max(values: Double*): Double = js.native
  def min(value1: Int, values: Int*): Int = js.native
  def min(values: Double*): Double = js.native
  def random(): Double = js.native

object URIUtils:
  @js.native @JSGlobal("encodeURIComponent")
  def encodeURIComponent(uriComponent: String): String = js.native
  @js.native @JSGlobal("decodeURIComponent")
  def decodeURIComponent(encodedURIComponent: String): String = js.native
  @js.native @JSGlobal("encodeURI")
  def encodeURI(uri: String): String = js.native
  @js.native @JSGlobal("decodeURI")
  def decodeURI(encodedURI: String): String = js.native

/** What a caught foreign JS value is wrapped in; its field is `exception`. */
type JavaScriptException = _root_.js.JavaScriptException

object JavaScriptException:
  def apply(exception: scala.Any): JavaScriptException = _root_.js.JavaScriptException(exception)

@js.native
@JSGlobal("Error")
class Error(messageArg: String = js.native) extends Object:
  val name: String = js.native
  val message: String = js.native
  val stack: String = js.native
  val cause: scala.Any = js.native

/** A weak reference to `targetNo`, JavaScript's `WeakRef`. */
@js.native
@JSGlobal("WeakRef")
class WeakRef[+T](targetNo: T) extends Object:
  def deref[S >: T](): UndefOr[S] = js.native

/** JavaScript's `FinalizationRegistry`: the callback gets the held value of an object collected. */
@js.native
@JSGlobal("FinalizationRegistry")
class FinalizationRegistry[-A, -B, -C](finalizationCallback: Function1[B, scala.Any]) extends Object:
  def register(theObject: A, heldValue: B, unregistrationToken: C): Unit = js.native
  def register(theObject: A, heldValue: B): Unit = js.native
  def unregister(unregistrationToken: C): Boolean = js.native

// Scala.js's `JSStringOps`, through `import js.JSStringOps._`: a string's JavaScript methods, those whose names
// Scala's `String` gives another meaning taking `js` before them (`jsSplit` for `split`).
object JSStringOps:
  implicit def enableJSStringOps(x: String): JSStringOps = new JSStringOps(x)

final class JSStringOps(self: String):
  def jsIndexOf(searchString: String, position: Int): Int = stringMethod2(self, "indexOf", searchString, position).asInstanceOf[Int]
  def jsIndexOf(searchString: String): Int = stringMethod1(self, "indexOf", searchString).asInstanceOf[Int]
  def jsLastIndexOf(searchString: String, position: Int): Int = stringMethod2(self, "lastIndexOf", searchString, position).asInstanceOf[Int]
  def jsLastIndexOf(searchString: String): Int = stringMethod1(self, "lastIndexOf", searchString).asInstanceOf[Int]
  def localeCompare(that: String): Int = stringMethod1(self, "localeCompare", that).asInstanceOf[Int]
  def `match`(regexp: String): Array[String] = stringMethod1(self, "match", regexp).asInstanceOf[Array[String]]
  def `match`(regexp: RegExp): Array[String] = stringMethod1(self, "match", regexp).asInstanceOf[Array[String]]
  def jsReplace(searchValue: String, replaceValue: String): String = stringMethod2(self, "replace", searchValue, replaceValue).asInstanceOf[String]
  def jsReplace(searchValue: RegExp, replaceValue: String): String = stringMethod2(self, "replace", searchValue, replaceValue).asInstanceOf[String]
  def search(regexp: String): Int = stringMethod1(self, "search", regexp).asInstanceOf[Int]
  def search(regexp: RegExp): Int = stringMethod1(self, "search", regexp).asInstanceOf[Int]
  def jsSlice(start: Int, end: Int): String = stringMethod2(self, "slice", start, end).asInstanceOf[String]
  def jsSlice(start: Int): String = stringMethod1(self, "slice", start).asInstanceOf[String]
  def jsSplit(separator: String, limit: Int): Array[String] = stringMethod2(self, "split", separator, limit).asInstanceOf[Array[String]]
  def jsSplit(separator: String): Array[String] = stringMethod1(self, "split", separator).asInstanceOf[Array[String]]
  def jsSplit(separator: RegExp, limit: Int): Array[String] = stringMethod2(self, "split", separator, limit).asInstanceOf[Array[String]]
  def jsSplit(separator: RegExp): Array[String] = stringMethod1(self, "split", separator).asInstanceOf[Array[String]]
  def jsSubstring(start: Int, end: Int): String = stringMethod2(self, "substring", start, end).asInstanceOf[String]
  def jsSubstring(start: Int): String = stringMethod1(self, "substring", start).asInstanceOf[String]
  def toLocaleLowerCase(): String = stringMethod0(self, "toLocaleLowerCase").asInstanceOf[String]
  def toLocaleUpperCase(): String = stringMethod0(self, "toLocaleUpperCase").asInstanceOf[String]
  def normalize(): String = stringMethod0(self, "normalize").asInstanceOf[String]

@js("$0[$1]()")
private[js] def stringMethod0(s: String, name: String): scala.Any
@js("$0[$1]($2)")
private[js] def stringMethod1(s: String, name: String, a: scala.Any): scala.Any
@js("$0[$1]($2, $3)")
private[js] def stringMethod2(s: String, name: String, a: scala.Any, b: scala.Any): scala.Any

@js.native
@JSGlobal("RegExp")
class RegExp(pattern: String, flags: String = js.native) extends Object:
  def test(text: String): Boolean = js.native
  def exec(text: String): Array[String] = js.native
  val source: String = js.native
  val global: Boolean = js.native
  val ignoreCase: Boolean = js.native
  val multiline: Boolean = js.native
  var lastIndex: Int = js.native
