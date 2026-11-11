// `compareBaseClassesLength` (Implicits.scala:1564-1571) counts a top-level given's owner, its file's `$package` object
// (a module class whose parent is Object, Desugar.scala:1964-1967), as more base classes than a universal trait's two
// (`U`, `Any`), so of one level (the import shares the package's owner) and arity 0 `sort` puts `a` first and scalac
// 3.8.4 names a before b. master (efddb38de3) and the head (61c1af7941) "both b and a provide pick on R"; push 5 "both
// a and b", in its own report at the selection. Queued: push 6 (typeMismatch whole).
package p

class R
trait Ops:
  extension (r: R) def pick: String
class Impl(n: String) extends Ops:
  extension (r: R) def pick: String = n
given a: Ops = Impl("a")
trait U extends Any:
  inline given b: Ops = Impl("b")
object V extends U
import V.b

object Main:
  def main(args: Array[String]): Unit = println((new R).pick)
