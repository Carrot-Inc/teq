// The members of the builtin classes that no TASTy file defines: what an array is indexed and
// measured by, and the hash of a value. They stay under `--std=scala-library`.
package scala

extension [T](a: Array[T])
  @js("$0[$1]")
  @jvm("$0 $1:I array_apply")
  def apply(i: Int): T
  @js("$0[$1] = $2")
  @jvm("$0 $1:I $2 array_update")
  def update(i: Int, value: T): Unit
  @js("$0.length")
  @jvm("$0 array_length")
  def length: Int
  @js("$0.slice()")
  @jvm("$0 array_clone")
  def clone(): Array[T]
  @js("$0.push($1)")
  def push(value: T): Int

// Between a buffer and an array, which are one value in JavaScript: the buffer's elements as an
// array of the kind the tag names, as an array of references (what scalac's untagged
// collections hold), as an array of the kind of another, and an array's elements in a buffer.
// The growable sequence the collections are built on is the buffer, `RawBuffer`, which
// std/buffer.scala and std/jvm_buffer.scala define for their targets with what goes between
// it and an array. The definitions the std shares keep the names JavaScript's text calls them
// by (`iterableToArray`, `fromArray`), whichever of the two they take.

// Scala.js's `js.Array` members that Scala's `Array` lacks, on the JS array both are, with the
// JavaScript semantics: `reverse` in place is `reverseInPlace` (Scala's `reverse` copies), and
// `sort()` without a function compares as strings.
extension [T](a: Array[T])
  @js("$0.reverse()")
  def reverseInPlace(): Array[T]
  @js("$0.sort()")
  def sort(): Array[T]
  @js("$0.sort($1)")
  def sort(compareFn: (T, T) => Int): Array[T]
  @js("$0.length = $1")
  def length_=(n: Int): Unit
  @js("$0.pop()")
  def pop(): T
  @js("$0.shift()")
  def shift(): T
  @js("$0.push(...$1)")
  def push(items: T*): Int
  @js("$0.unshift(...$1)")
  def unshift(items: T*): Int
  @js("$0.splice($1, $2, ...$3)")
  def splice(index: Int, deleteCount: Int, items: T*): Array[T]
  @js("$0.slice($1)")
  def jsSlice(start: Int): Array[T]
  @js("$0.concat(...$1)")
  def concat[B >: T](items: Array[? <: B]*): Array[B]
  @js("$0.lastIndexOf($1)")
  def lastIndexOf(elem: T): Int
  @js("$0.join()")
  def join(): String
  @js("$0.join($1)")
  def join(separator: String): String

// `f"..."` under `--std=scala-library`, whose `StringContext.f` the compiler expands: the parts
// with `%s` for a hole without a specifier, formatted as `String.format` formats.
@js("$formatInterpolated($0, $1)")
def formatInterpolation(parts: Array[String], args: Array[Any]): String = scala.runtime.formatInterpolatedParts(parts, args)

@js("[]")
@jvm("new java/util/ArrayList dup invokespecial java/util/ArrayList.<init>()V")
def emptyArray[T]: RawBuffer[T]

/** `new Array[T](length)`: `length` copies of the zero of `T`. */
@js("new Array($0).fill($1)")
@jvm("$0:I array_new")
def newArray[T](length: Int, zero: T): Array[T] =
  val out = emptyArray[T]
  var i = 0
  while i < length do
    out.push(zero)
    i += 1
  untaggedArray(out)

extension [A](a: A)
  @js("$hash($0)")
  @jvm("getstatic scala/runtime/jvm$package$.MODULE$:Lscala/runtime/jvm$package$; $0:L invokevirtual scala/runtime/jvm$package$.anyHash(Ljava/lang/Object;)I")
  def ## : Int

@js("$0")
@jvm("$0:L")
def unsafeCast[A, B](a: A): B

// The members of the builtin function and tuple classes that are no compiler intrinsics: a
// function value is a JavaScript function, so these are extensions, whichever std is in.
extension [A, B](f: A => B)
  def andThen[C](g: B => C): A => C = x => g(f(x))
  def compose[C](g: C => A): C => B = x => f(g(x))

extension [A, B](p: (A, B))
  def swap: (B, A) = (p._2, p._1)

// The array helpers the standard library is built on, in the file both std modes share: the
// layer's `IArray` needs them under `--std=scala-library` too.
def iterableToArray[A](xs: IterableOnce[A]): RawBuffer[A] =
  val out = emptyArray[A]
  xs.foreach(x => out.push(x))
  out

@js("$0.slice()")
@jvm("new java/util/ArrayList dup $0 invokespecial java/util/ArrayList.<init>(Ljava/util/Collection;)V")
def copyArray[A, B >: A](arr: RawBuffer[A]): RawBuffer[B]

// A sequence spread into the varargs of a Java method (`Stream.of(xs*)`) is a copy of its elements,
// the array dotty's `ElimRepeated.adaptToArray` makes of it (`Arrays.seqToArray`), where an array
// spread is the array itself.
def javaVarargs[A](xs: Seq[A]): Seq[A] = scala.collection.immutable.ArraySeq.unsafeWrapArray(untaggedArray(iterableToArray(xs)))
