package r2nullreturn

// An explicit `toString` is a call whose result is what the method returns, `null` included
// (dotty's `BCodeBodyBuilder.genApply`), apart from a concatenation's rendering
// (`genStringConcat`): the second call of `x.toString.toString` on an `Empty` throws, as does
// `toString` of a null array, whole and over the products alike.
class Empty:
  override def toString: String = null

object Api:
  inline def twice(x: Any): String = x.toString.toString
  inline def arrayString(x: Array[Int]): String = x.toString
  def result(x: Any): Boolean = x.toString == null
