// Erasure as dotty's `TypeErasure` has it: a `*:` chain as an upper bound erases as the chain spelled out does, to
// `Product` (master: the tuple class of its elements; a chain behind an alias is left with E1); a union of `String`
// and a class to their erased lub, `CharSequence` of `String | java.lang.StringBuilder`, `Serializable` of
// `String | List[Int]` (master: `Object`), by the first operand's linearisation, `Both | String` to `Serializable`
// where `String | Both` is `CharSequence`, an array of either the array of the lub; an applied opaque type to its
// underlying type over its arguments, `Arr[Int]` of `opaque type Arr[A] = Array[A]` to `[I`, where a generic one
// stays `Object`.
// abi: API#bound API#direct API#union API#serial API#rserial API#left API#right API#arrays API#nullable API#prim API#spec API#gen
package erasure
opaque type Arr[A] = Array[A]
object Arr { def ints(n: Int): Arr[Int] = new Array[Int](n) }
class Both extends CharSequence with java.io.Serializable:
  def length: Int = 0
  def charAt(i: Int): Char = 'a'
  def subSequence(a: Int, b: Int): CharSequence = ""
class API:
  def bound[T <: Int *: EmptyTuple](x: T): T = x
  def direct(x: Int *: String *: EmptyTuple): Int *: String *: EmptyTuple = x
  def union(x: String | java.lang.StringBuilder): String | java.lang.StringBuilder = x
  def serial(x: String | List[Int]): String | List[Int] = x
  def rserial(x: List[Int] | String): List[Int] | String = x
  def left(x: String | Both): String | Both = x
  def right(x: Both | String): Both | String = x
  def arrays(x: Array[String] | Array[Both]): Array[String] | Array[Both] = x
  def nullable(x: String | Null): String | Null = x
  def prim(x: String | Int): String | Int = x
  def spec(x: Arr[Int]): Arr[Int] = x
  def gen[A](x: Arr[A]): Arr[A] = x
