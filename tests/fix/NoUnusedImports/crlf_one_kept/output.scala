// A clause keeping one selector is written as its import (dotty's `checkImports`), whose edit ends between the
// CR and the LF (`editPosAt` takes the CR for blank): the line keeps its CRLF.
package p
import scala.collection.mutable
object CRLF { val x = mutable.ArrayBuffer(1) }
