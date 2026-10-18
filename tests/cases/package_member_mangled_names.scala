// A class, an object or a package is found by its source name and by its encoded one alike, and by any spelling of
// one encoded key (`a-b$plusc`, `a$minusb$plusc`): dotty keeps the members of the root and the empty package under
// their mangled names (SymbolLoaders.PackageScope, name.mangled, NameTransformer.encode).
package `m-n`:
  object X:
    val v = "package by its encoded name"
package q$minusr:
  object Y:
    val v = "package by its source name"
object `a-b`:
  val v = "object by its encoded name"
object c$minusd:
  val w = "object by its source name"
class `k-l`:
  override def toString = "class by its encoded name"
object `a-b$plusc`:
  val v = "object by a spelling of one encoded key"
object Main:
  def main(args: Array[String]): Unit =
    println(m$minusn.X.v)
    println(`q-r`.Y.v)
    println(a$minusb.v)
    println(`c-d`.w)
    println(new k$minusl)
    println(a$minusb$plusc.v)
