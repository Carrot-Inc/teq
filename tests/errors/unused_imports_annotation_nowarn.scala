// A definition's own @nowarn silences the imports of its annotations' arguments.
package annotation_nowarn
object Lib { val n = 1 }
class Ann(x: Int) extends scala.annotation.StaticAnnotation
@Ann({ import Lib.n; 0 }) @scala.annotation.nowarn class C

import scala.collection.mutable.Stack

// teq: --werror --wunused imports
// expect: unused_imports_annotation_nowarn.scala:7:33: warning: unused import
