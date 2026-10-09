// A Conversion given with a using clause (`given [A](using Put[A]): Conversion[A, Elem]`) is not
// applied by teq: `type mismatch: found Int, required Elem` at every argument. Monomorphic
// Conversion givens are applied. scalac prints `Arg(i5)`, `a i1 b sx c btrue`.
package probe.conv

enum Elem:
  case Arg(text: String)
trait Put[A]:
  def param(a: A): String
object Put:
  given Put[Int] = i => "i" + i
  given Put[String] = s => "s" + s
object Elem:
  given fromPut[A](using put: Put[A]): Conversion[A, Elem] = a => Elem.Arg(put.param(a))
  given fromBool: Conversion[Boolean, Elem] = b => Elem.Arg("b" + b)
extension (sc: StringContext)
  def sql(args: Elem*): String = sc.parts.zipAll(args.map { case Elem.Arg(t) => t }, "", "").map(_ + _).mkString
object Main:
  def main(args: Array[String]): Unit =
    val f: Elem = 5
    println(f)
    println(sql"a ${1} b ${"x"} c ${true}")
