// An extension a library declares at the top level of its package (a member of the file's package object on the
// class path) uses the package's import, as ip4s's interpolators do; the import beside it is unused.
import com.comcast.ip4s.*
import scala.collection.mutable

object Main {
  def address = ip"127.0.0.1"
  def endpoint = port"8080"
}

// jars: scala-library ip4s-core literally cats-core cats-kernel
// teq: --werror --wunused imports --target jvm
// expect: unused_imports_classpath_toplevel.scala:4:25: warning: unused import
// absent: unused_imports_classpath_toplevel.scala:3:
