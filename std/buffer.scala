package scala

// The growable sequence the collections are built on, where the output is JavaScript or the
// interpreter runs: JavaScript's array, which grows, so every member of `Array` serves it and
// what goes between the two is the value itself. std/jvm_buffer.scala has the JVM's, where an
// array's length is fixed and the two are two classes.
type RawBuffer[T] = Array[T]

inline def emptyBuffer[T]: RawBuffer[T] = emptyArray[T]

inline def filledBuffer[T](inline n: Int)(inline elem: => T): RawBuffer[T] = Array.fill(n)(elem)

inline def tabulatedBuffer[T](inline n: Int)(inline f: Int => T): RawBuffer[T] = Array.tabulate(n)(f)

/** `n` times `zero`, what a node of fixed size starts as. */
inline def sizedBuffer[T](inline n: Int, inline zero: T): RawBuffer[T] = newArray[T](n, zero)

inline def arrayIterator[A](inline items: Array[A]): Iterator[A] = Iterator.over(items)

/** The buffer's elements as an array of the kind the tag names on the JVM. */
inline def taggedArray[T](inline b: RawBuffer[T]): Array[T] = b

/** The array an `@jvmEvidence` member made, as the tag its call passed makes it: `null` or a standard
  * tag (`ClassTag.Int`, `ClassTag(classOf[C])`, `standardAllocation`), which the JavaScript output passes
  * as `null`, keeps the array, as their `newArray` would allocate it; any other tag's `newArray` of the
  * array's length holds the elements, scala-library's one allocation of a known length. */
@jvmEvidence
def evidenced[T](tag: scala.reflect.ClassTag[T], items: Array[T]): Array[T] =
  if tag == null || tag.standardAllocation then items else copiedInto(tag.newArray(items.length), items)

/** The array scala-library's `ArrayBuilder.make` builds of the elements `run` adds one by one and in
  * collections (`addOne`, `addAll`), in its order: a builder of a reference class asks its tag for its
  * first array when the first element is added, of 16 elements or of the next power of two that holds
  * a collection of known size (`ofRef.ensureSize`), grows by `Arrays.copyOf`, and gives the tag's array
  * where the elements fill it, else a copy of their number (`ofRef.result`), `newArray(0)` for none; a
  * builder of a primitive class asks its tag nothing, and neither does `null` or a standard tag. */
@jvmEvidence
def builtByTag[T](tag: scala.reflect.ClassTag[T])(run: (T => Unit, IterableOnce[T] => Unit) => Unit): Array[T] =
  if tag == null || tag.standardAllocation || tag.runtimeClass.isPrimitive then
    val out = emptyBuffer[T]
    run(x => out.push(x), xs => xs.iterator.foreach(x => out.push(x)))
    taggedArray(out)
  else
    // `ofRef`'s state: each element written into the backing array as it is added (`addOne`), the array
    // grown by copying it (`resize`, `Arrays.copyOf`), the result that array where the elements fill it,
    // else a copy of their number (`result`, `mkArray`).
    var elems: Array[T] = null
    var capacity = 0
    var size = 0
    def ensureSize(n: Int): Unit =
      if capacity < n || capacity == 0 then
        var newSize = if capacity == 0 then 16 else capacity * 2
        while newSize < n do newSize *= 2
        elems = if elems == null then tag.newArray(newSize) else arrayCopyOf(elems, newSize)
        capacity = newSize
    def addOne(x: T): Unit =
      ensureSize(size + 1)
      elems(size) = x
      size += 1
    run(
      addOne,
      xs =>
        val k = xs.knownSize
        if k > 0 then ensureSize(size + k)
        xs.iterator.foreach(addOne)
    )
    if capacity != 0 && capacity == size then elems
    else if elems == null then tag.newArray(0)
    else arrayCopyOf(elems, size)

/** `java.util.Arrays.copyOf` of an array of references: its first `n` elements, `null` past its end. */
@js("$arrayCopyOf($0, $1)")
def arrayCopyOf[T](a: Array[T], n: Int): Array[T] =
  val out = newArray[T](n, unsafeCast(null))
  var i = 0
  while i < n && i < a.length do
    out(i) = a(i)
    i += 1
  out

/** scala-library's `Array.from` by the tag a call passed: of a known size, the tag's array of that size
  * first, then the elements read into it; else as `ArrayBuilder.make` builds it (`builtByTag`). */
@jvmEvidence
def fromByTag[T](tag: scala.reflect.ClassTag[T], source: IterableOnce[T]): Array[T] =
  val n = source.knownSize
  if n < 0 then builtByTag(tag)((add, _) => source.iterator.foreach(add))
  else if tag == null || tag.standardAllocation then taggedArray(iterableToArray(source))
  else
    val arr = tag.newArray(n)
    val it = source.iterator
    var i = 0
    while i < n do
      arr(i) = it.next()
      i += 1
    arr

/** A collection's `toArray` by the tag a call passed (`IterableOnceOps.toArray`, `Array.from`'s order):
  * `plain`, the collection's own copy, where no tag would see the allocation. */
inline def arrayByTag[T](tag: scala.reflect.ClassTag[T], source: IterableOnce[T], inline plain: Array[T]): Array[T] =
  if tag == null || tag.standardAllocation then plain else fromByTag(tag, source)

/** `items` copied into `out`, of its length at least: `out`. */
@js("Object.assign($0, $1)")
def copiedInto[T](out: Array[T], items: Array[T]): Array[T] =
  var i = 0
  while i < items.length do
    out(i) = items(i)
    i += 1
  out

/** As an array of references, what scalac's untagged collections hold. */
inline def untaggedArray[T](inline b: RawBuffer[T]): Array[T] = b

/** As an array of the kind of `other`. */
inline def arrayLike[T](inline b: RawBuffer[T], inline other: Array[?]): Array[T] = b

/** As an array of the kind of the `ArrayBuilder` that collected the elements. */
inline def builtArray[T](inline b: RawBuffer[T], inline builder: Any): Array[T] = b

/** An array's elements in a buffer, and in one of the caller's own. */
inline def buffered[T](inline a: Array[T]): RawBuffer[T] = a

inline def bufferedCopy[T](inline a: Array[T]): RawBuffer[T] = a.clone()

/** The elements in a buffer of the caller's own, which the std builds its results from: copied by the
  * collection's traversal, where `toArray` would ask the element's `ClassTag` of a member classes override. */
inline def rawItems[A](inline xs: IterableOps[A, [X] =>> Any, Any]): RawBuffer[A] = iterableToArray(xs)
