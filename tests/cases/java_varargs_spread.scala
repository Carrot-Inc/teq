//> using platform jvm
// jars: javafix
// An array spread into a Java varargs as it is written (`f(arr*)`) is the parameter's array
// itself, as scalac passes it: an array of references for any array of references the elements
// fit (`CharSequence...` given a `String` array), a primitive array for the same primitive. The
// callee's writes show in the caller's array, and a stream made of it sees the caller's later
// writes. A sequence the program wrote, a copy or a wrapper of an array among them, and an array
// whose elements the parameter boxes, go as a new array. `fix.Varargs` is a Java class of
// tests/classfile/fixtures.
import java.util.stream.{IntStream, Stream}

object Main:
  // The same spreads inside inline methods, whose expansions keep what each spread is.
  inline def setFirst(a: Array[String]): Unit = fix.Varargs.set(a*)
  inline def streamOf(a: Array[String]): Stream[String] = Stream.of[String](a*)
  inline def copiedStreamOf(a: Array[String]): Stream[String] = Stream.of[String](Predef.copyArrayToImmutableIndexedSeq(a)*)

  def main(args: Array[String]): Unit =
    val strings = Array("before", "second")
    fix.Varargs.set(strings*)
    println("CharSequence... given String[]: " + strings(0))
    val sequences = Array[CharSequence]("before", "second")
    fix.Varargs.set(sequences*)
    println("CharSequence... given CharSequence[]: " + sequences(0))
    val integers = Array[java.lang.Integer](1, 2, 3)
    println("Number... given Integer[]: " + fix.Varargs.count(integers*))
    val listed = List("before", "second")
    fix.Varargs.set(listed*)
    println("CharSequence... given a list: " + listed.head)
    val written = Array("before", "second")
    fix.Varargs.set(Predef.copyArrayToImmutableIndexedSeq(written)*)
    println("CharSequence... given a copy written: " + written(0))
    val inlined = Array("before", "second")
    setFirst(inlined)
    println("CharSequence... given String[] in an inline method: " + inlined(0))

    val direct = Array("before", "second")
    val s1 = Stream.of[String](direct*)
    direct(0) = "after"
    println("spread: " + s1.toList())
    val copied = Array("before", "second")
    val s2 = Stream.of[String](Predef.copyArrayToImmutableIndexedSeq(copied)*)
    copied(0) = "after"
    println("a copy written: " + s2.toList())
    val wrapped = Array("before", "second")
    val s3 = Stream.of[String](scala.runtime.ScalaRunTime.wrapRefArray(wrapped)*)
    wrapped(0) = "after"
    println("a wrapper written: " + s3.toList())
    val unsafe = Array("before", "second")
    val s4 = Stream.of[String](scala.collection.immutable.ArraySeq.unsafeWrapArray(unsafe)*)
    unsafe(0) = "after"
    println("an unsafe wrapper written: " + s4.toList())
    val throughInline = Array("before", "second")
    val s6 = streamOf(throughInline)
    throughInline(0) = "after"
    println("spread in an inline method: " + s6.toList())
    val copiedInline = Array("before", "second")
    val s7 = copiedStreamOf(copiedInline)
    copiedInline(0) = "after"
    println("a copy written in an inline method: " + s7.toList())

    val ints = Array(1, 2)
    val s5 = IntStream.of(ints*)
    ints(0) = 9
    println("int... given int[]: " + s5.sum())
    val boxed = Array(1, 2)
    val list = java.util.Arrays.asList(boxed*)
    boxed(0) = 9
    println("T... given int[]: " + list)
