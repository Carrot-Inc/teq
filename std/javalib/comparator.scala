// `java.util.Comparator` in a file of its own: the parent of the std's `Ordering`, as scala-library's
// `Ordering` extends it, which the first conversion search of a program reaches through `Numeric`,
// an `Ordering` too, so that naming it enters this file alone and not the rest of `java.util`, as
// dotty reads the interface from its own class file.
package java.util:

  @jvmClass("java/util/Comparator")
  trait Comparator[T]:
    def compare(a: T, b: T): Int

  @javaDefined
  @jvmClass("java/util/Comparator")
  object Comparator
