// The imports before a file's package blocks are the file's: used where any block uses them.
import scala.collection.mutable.ListBuffer
import scala.collection.mutable.ArrayBuffer

package a:
  object A { val l = ListBuffer(1) }

package b:
  import scala.collection.mutable.Queue
  object B { val x = 1 }

// teq: --werror --wunused imports
// expect: unused_imports_packages.scala:3:33: warning: unused import
// expect: unused_imports_packages.scala:9:35: warning: unused import
