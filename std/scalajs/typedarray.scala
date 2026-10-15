package scala.scalajs.js.typedarray

import scala.scalajs.js
import scala.scalajs.js.annotation.{JSBracketAccess, JSGlobal}

@js.native
@JSGlobal("ArrayBuffer")
class ArrayBuffer(size: Int) extends js.Object:
  def byteLength: Int = js.native
  def slice(begin: Int, end: Int = js.native): ArrayBuffer = js.native

@js.native
@JSGlobal("ArrayBuffer")
object ArrayBuffer extends js.Object:
  def isView(arg: scala.Any): Boolean = js.native

@js.native
trait ArrayBufferView extends js.Object:
  def buffer: ArrayBuffer = js.native
  def byteLength: Int = js.native
  def byteOffset: Int = js.native

// An iterable of its elements, as Scala.js's (`IArray.from(new Int8Array(buffer))`).
@js.native
trait TypedArray[T, Repr] extends ArrayBufferView, js.Iterable[T]:
  def length: Int = js.native
  @JSBracketAccess
  def apply(index: Int): T = js.native
  @JSBracketAccess
  def update(index: Int, value: T): Unit = js.native
  def set(array: Repr | js.Array[T] | scala.Array[T], offset: Int = js.native): Unit = js.native
  def subarray(begin: Int, end: Int = js.native): Repr = js.native
  def slice(begin: Int = js.native, end: Int = js.native): Repr = js.native
  def fill(value: T, begin: Int = js.native, end: Int = js.native): Repr = js.native

@js.native
@JSGlobal("DataView")
class DataView(buffer: ArrayBuffer, byteOffset: Int = js.native, byteLength: Int = js.native) extends ArrayBufferView:
  def getInt8(byteOffset: Int): Byte = js.native
  def getUint8(byteOffset: Int): Short = js.native
  def getInt16(byteOffset: Int, littleEndian: Boolean = js.native): Short = js.native
  def getUint16(byteOffset: Int, littleEndian: Boolean = js.native): Int = js.native
  def getInt32(byteOffset: Int, littleEndian: Boolean = js.native): Int = js.native
  def getUint32(byteOffset: Int, littleEndian: Boolean = js.native): Double = js.native
  def getFloat32(byteOffset: Int, littleEndian: Boolean = js.native): Float = js.native
  def getFloat64(byteOffset: Int, littleEndian: Boolean = js.native): Double = js.native
  def setInt8(byteOffset: Int, value: Byte): Unit = js.native
  def setUint8(byteOffset: Int, value: Short): Unit = js.native
  def setInt16(byteOffset: Int, value: Short, littleEndian: Boolean = js.native): Unit = js.native
  def setUint16(byteOffset: Int, value: Int, littleEndian: Boolean = js.native): Unit = js.native
  def setInt32(byteOffset: Int, value: Int, littleEndian: Boolean = js.native): Unit = js.native
  def setUint32(byteOffset: Int, value: Double, littleEndian: Boolean = js.native): Unit = js.native
  def setFloat32(byteOffset: Int, value: Float, littleEndian: Boolean = js.native): Unit = js.native
  def setFloat64(byteOffset: Int, value: Double, littleEndian: Boolean = js.native): Unit = js.native

/** Scala.js's constructors: a length, another typed array of the kind, a JS iterable of the elements (a
  * `js.Array`), or a buffer with an offset and a length. The element types are Scala.js's: `Byte` of an
  * `Int8Array`, `Short` of a `Uint8Array`, `Float` of a `Float32Array`; each companion is the JavaScript
  * constructor, with its `BYTES_PER_ELEMENT`. */
@js.native
@JSGlobal("Int8Array")
class Int8Array private[this] () extends TypedArray[Byte, Int8Array]:
  def this(length: Int) = this()
  def this(typedArray: Int8Array) = this()
  def this(array: js.Iterable[Byte]) = this()
  def this(buffer: ArrayBuffer, byteOffset: Int = 0, length: Int = ???) = this()

@js.native
@JSGlobal("Int8Array")
object Int8Array extends js.Object:
  val BYTES_PER_ELEMENT: Int = js.native

@js.native
@JSGlobal("Uint8Array")
class Uint8Array private[this] () extends TypedArray[Short, Uint8Array]:
  def this(length: Int) = this()
  def this(typedArray: Uint8Array) = this()
  def this(array: js.Iterable[Short]) = this()
  def this(buffer: ArrayBuffer, byteOffset: Int = 0, length: Int = ???) = this()

@js.native
@JSGlobal("Uint8Array")
object Uint8Array extends js.Object:
  val BYTES_PER_ELEMENT: Int = js.native

@js.native
@JSGlobal("Uint8ClampedArray")
class Uint8ClampedArray private[this] () extends TypedArray[Int, Uint8ClampedArray]:
  def this(length: Int) = this()
  def this(typedArray: Uint8ClampedArray) = this()
  def this(array: js.Iterable[Int]) = this()
  def this(buffer: ArrayBuffer, byteOffset: Int = 0, length: Int = ???) = this()

@js.native
@JSGlobal("Uint8ClampedArray")
object Uint8ClampedArray extends js.Object:
  val BYTES_PER_ELEMENT: Int = js.native

@js.native
@JSGlobal("Int16Array")
class Int16Array private[this] () extends TypedArray[Short, Int16Array]:
  def this(length: Int) = this()
  def this(typedArray: Int16Array) = this()
  def this(array: js.Iterable[Short]) = this()
  def this(buffer: ArrayBuffer, byteOffset: Int = 0, length: Int = ???) = this()

@js.native
@JSGlobal("Int16Array")
object Int16Array extends js.Object:
  val BYTES_PER_ELEMENT: Int = js.native

@js.native
@JSGlobal("Uint16Array")
class Uint16Array private[this] () extends TypedArray[Int, Uint16Array]:
  def this(length: Int) = this()
  def this(typedArray: Uint16Array) = this()
  def this(array: js.Iterable[Int]) = this()
  def this(buffer: ArrayBuffer, byteOffset: Int = 0, length: Int = ???) = this()

@js.native
@JSGlobal("Uint16Array")
object Uint16Array extends js.Object:
  val BYTES_PER_ELEMENT: Int = js.native

@js.native
@JSGlobal("Int32Array")
class Int32Array private[this] () extends TypedArray[Int, Int32Array]:
  def this(length: Int) = this()
  def this(typedArray: Int32Array) = this()
  def this(array: js.Iterable[Int]) = this()
  def this(buffer: ArrayBuffer, byteOffset: Int = 0, length: Int = ???) = this()

@js.native
@JSGlobal("Int32Array")
object Int32Array extends js.Object:
  val BYTES_PER_ELEMENT: Int = js.native

@js.native
@JSGlobal("Uint32Array")
class Uint32Array private[this] () extends TypedArray[Double, Uint32Array]:
  def this(length: Int) = this()
  def this(typedArray: Uint32Array) = this()
  def this(array: js.Iterable[Double]) = this()
  def this(buffer: ArrayBuffer, byteOffset: Int = 0, length: Int = ???) = this()

@js.native
@JSGlobal("Uint32Array")
object Uint32Array extends js.Object:
  val BYTES_PER_ELEMENT: Int = js.native

@js.native
@JSGlobal("Float32Array")
class Float32Array private[this] () extends TypedArray[Float, Float32Array]:
  def this(length: Int) = this()
  def this(typedArray: Float32Array) = this()
  def this(array: js.Iterable[Float]) = this()
  def this(buffer: ArrayBuffer, byteOffset: Int = 0, length: Int = ???) = this()

@js.native
@JSGlobal("Float32Array")
object Float32Array extends js.Object:
  val BYTES_PER_ELEMENT: Int = js.native

@js.native
@JSGlobal("Float64Array")
class Float64Array private[this] () extends TypedArray[Double, Float64Array]:
  def this(length: Int) = this()
  def this(typedArray: Float64Array) = this()
  def this(array: js.Iterable[Double]) = this()
  def this(buffer: ArrayBuffer, byteOffset: Int = 0, length: Int = ???) = this()

@js.native
@JSGlobal("Float64Array")
object Float64Array extends js.Object:
  val BYTES_PER_ELEMENT: Int = js.native

// The object names are those of the original's implicit classes.
object ByteArrayOps:
  extension (bytes: scala.Array[Byte])
    @js("new Int8Array($0)")
    def toTypedArray: Int8Array

object Int8ArrayOps:
  extension (array: Int8Array)
    @js("Array.from($0)")
    def toArray: scala.Array[Byte]

given AB2TA: ByteArrayOps.type = ByteArrayOps

// A `ByteBuffer` over a typed array, which the std's buffers never are: `typedArray` gives a copy
// of the heap array.
final class TypedArrayBufferOps(buffer: java.nio.ByteBuffer):
  def hasTypedArray(): Boolean = false
  def typedArray(): Int8Array = ByteArrayOps.toTypedArray(buffer.array())

object TypedArrayBufferOps:
  implicit def byteBufferOps(buffer: java.nio.ByteBuffer): TypedArrayBufferOps = new TypedArrayBufferOps(buffer)
given TA2AB: Int8ArrayOps.type = Int8ArrayOps
