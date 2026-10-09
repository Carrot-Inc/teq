package scala

// The growable sequence the collections are built on, on the JVM: the JDK's list, which owns
// its capacity. `Array` is the JVM's array, whose length is fixed; std/buffer.scala has the
// buffer of JavaScript and the interpreter, the array itself. The bodies beside the templates
// are what a macro's interpreter runs, whose arrays grow.
@jvmClass("java/util/ArrayList")
final class RawBuffer[T]

@jvm("new java/util/ArrayList dup invokespecial java/util/ArrayList.<init>()V")
def emptyBuffer[T]: RawBuffer[T] = emptyArray[T]

@jvm("new java/util/ArrayList dup $0:I invokespecial java/util/ArrayList.<init>(I)V")
private def bufferOfCapacityAtLeast[T](n: Int): RawBuffer[T] = emptyArray[T]

/** An empty buffer with room for `n` elements; none for a negative `n`, as a loop to `n` adds none. */
def bufferOfCapacity[T](n: Int): RawBuffer[T] = bufferOfCapacityAtLeast[T](if n < 0 then 0 else n)

/** `n` times `zero`, what a node of fixed size starts as. */
@jvm("new java/util/ArrayList dup $0:I $1:L invokestatic java/util/Collections.nCopies(ILjava/lang/Object;)Ljava/util/List; invokespecial java/util/ArrayList.<init>(Ljava/util/Collection;)V")
def sizedBuffer[T](n: Int, zero: T): RawBuffer[T] = filledBuffer(n)(zero)

def filledBuffer[T](n: Int)(elem: => T): RawBuffer[T] =
  val out = bufferOfCapacity[T](n)
  var i = 0
  while i < n do
    out.push(elem)
    i += 1
  out

def tabulatedBuffer[T](n: Int)(f: Int => T): RawBuffer[T] =
  val out = bufferOfCapacity[T](n)
  var i = 0
  while i < n do
    out.push(f(i))
    i += 1
  out

extension [T](b: RawBuffer[T])
  @jvm("invokevirtual java/util/ArrayList.get(I)Ljava/lang/Object;")
  def apply(i: Int): T = unsafeCast[RawBuffer[T], Array[T]](b)(i)
  @jvm("$0 $1:I $2:L invokevirtual java/util/ArrayList.set(ILjava/lang/Object;)Ljava/lang/Object; pop")
  def update(i: Int, value: T): Unit = unsafeCast[RawBuffer[T], Array[T]](b)(i) = value
  @jvm("invokevirtual java/util/ArrayList.size()I")
  def length: Int = unsafeCast[RawBuffer[T], Array[T]](b).length
  @jvm("new java/util/ArrayList dup $0 invokespecial java/util/ArrayList.<init>(Ljava/util/Collection;)V")
  def clone(): RawBuffer[T] = unsafeCast(unsafeCast[RawBuffer[T], Array[T]](b).clone())
  @jvm("$0 dup $1:L invokevirtual java/util/ArrayList.add(Ljava/lang/Object;)Z pop invokevirtual java/util/ArrayList.size()I")
  def push(value: T): Int = unsafeCast[RawBuffer[T], Array[T]](b).push(value)

/** The buffer's elements as an array of the kind the tag names. */
@jvm("rt $0 $1 rtcall arrayOfTag(Ljava/util/ArrayList;Lscala/reflect/ClassTag;)Ljava/lang/Object; cast_result")
def taggedArray[T: scala.reflect.ClassTag](b: RawBuffer[T]): Array[T] = unsafeCast(b)

/** As an array of references, what scalac's untagged collections hold. */
@jvm("$0 invokevirtual java/util/ArrayList.toArray()[Ljava/lang/Object;")
def untaggedArray[T](b: RawBuffer[T]): Array[T] = unsafeCast(b)

/** As an array of the kind of `other`. */
@jvm("rt $0 $1:L rtcall arrayLikeOf(Ljava/util/ArrayList;Ljava/lang/Object;)Ljava/lang/Object; cast_result")
def arrayLike[T](b: RawBuffer[T], other: Array[?]): Array[T] = unsafeCast(b)

/** An array's elements in a buffer, and in one of the caller's own. */
@jvm("rt $0:L rtcall bufferOfArray(Ljava/lang/Object;)Ljava/util/ArrayList;")
def buffered[T](a: Array[T]): RawBuffer[T] = unsafeCast(a)

@jvm("rt $0:L rtcall bufferOfArray(Ljava/lang/Object;)Ljava/util/ArrayList;")
def bufferedCopy[T](a: Array[T]): RawBuffer[T] = unsafeCast(a.clone())

// What the std reads its own buffers with: the members of `Array` it calls on them, which
// JavaScript's array serves there.
extension [T](b: RawBuffer[T])
  def foreach[U](f: T => U): Unit =
    var i = 0
    while i < b.length do
      f(b(i))
      i += 1
  def map[B](f: T => B): RawBuffer[B] =
    val out = emptyBuffer[B]
    var i = 0
    while i < b.length do
      out.push(f(b(i)))
      i += 1
    out
  def filter(p: T => Boolean): RawBuffer[T] =
    val out = emptyBuffer[T]
    var i = 0
    while i < b.length do
      if p(b(i)) then out.push(b(i))
      i += 1
    out
  def exists(p: T => Boolean): Boolean =
    var i = 0
    var found = false
    while !found && i < b.length do
      found = p(b(i))
      i += 1
    found
  def forall(p: T => Boolean): Boolean = !b.exists(x => !p(x))
  def isEmpty: Boolean = b.length == 0
  def nonEmpty: Boolean = b.length != 0
