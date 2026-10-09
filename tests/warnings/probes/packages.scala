// The imports before a file's package blocks are the file's: used where any block uses them.
import scala.collection.mutable.ListBuffer
import scala.collection.mutable.ArrayBuffer

package a:
  object A { val l = ListBuffer(1) }

package b:
  import scala.collection.mutable.Queue
  object B { val x = 1 }
