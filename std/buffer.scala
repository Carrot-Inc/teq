package scala

// The growable sequence the collections are built on, where the output is JavaScript or the
// interpreter runs: JavaScript's array, which grows, so every member of `Array` serves it and
// what goes between the two is the value itself. std/jvm_buffer.scala has the JVM's, where an
// array's length is fixed and the two are two classes.
type RawBuffer[T] = Array[T]

inline def emptyBuffer[T]: RawBuffer[T] = Array.empty[T]

inline def filledBuffer[T](inline n: Int)(inline elem: => T): RawBuffer[T] = Array.fill(n)(elem)

inline def tabulatedBuffer[T](inline n: Int)(inline f: Int => T): RawBuffer[T] = Array.tabulate(n)(f)

/** `n` times `zero`, what a node of fixed size starts as. */
inline def sizedBuffer[T](inline n: Int, inline zero: T): RawBuffer[T] = newArray[T](n, zero)

inline def arrayIterator[A](inline items: Array[A]): Iterator[A] = Iterator.over(items)

/** The buffer's elements as an array of the kind the tag names on the JVM. */
inline def taggedArray[T](inline b: RawBuffer[T]): Array[T] = b

/** As an array of references, what scalac's untagged collections hold. */
inline def untaggedArray[T](inline b: RawBuffer[T]): Array[T] = b

/** As an array of the kind of `other`. */
inline def arrayLike[T](inline b: RawBuffer[T], inline other: Array[?]): Array[T] = b

/** As an array of the kind of the `ArrayBuilder` that collected the elements. */
inline def builtArray[T](inline b: RawBuffer[T], inline builder: Any): Array[T] = b

/** An array's elements in a buffer, and in one of the caller's own. */
inline def buffered[T](inline a: Array[T]): RawBuffer[T] = a

inline def bufferedCopy[T](inline a: Array[T]): RawBuffer[T] = a.clone()

/** The elements in a buffer of the caller's own, which the std builds its results from. */
inline def rawItems[A](inline xs: IterableOps[A, [X] =>> Any, Any]): RawBuffer[A] = xs.toArray
