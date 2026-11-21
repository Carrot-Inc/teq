package kbt

// A trait whose implicit clause is first, applied with a using list in a body the library keeps: the reference's
// constructor holds an empty clause before it (NamerOps.normalizeIfConstructor), so the written list is the pickle's
// second; over the library's products the program prints the arguments written, the given beside them unused.
trait Tr(implicit a: Int, b: Int):
  def value: Int = a + b
trait Tn(implicit a: Int, b: String, c: Long):
  def value: String = s"$a $b $c"

object Lib:
  given Int = 20
  given String = "g"
  given Long = 300L
  def short: Int = (new Tr(using 10) {}).value
  def whole: Int = (new Tr(using 10, 5) {}).value
  class P extends Tr(using 10)
  def parent: Int = (new P).value
  val values: String = (new Tn(using 1, "w", 3L) {}).value
