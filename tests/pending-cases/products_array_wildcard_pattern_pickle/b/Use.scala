// A downstream of a/ as tests/modules builds it. scalac 3.8.4 prints the .expected, and so does each of teq's runs.
// scalac's -from-tasty read of teq's pickle of a/ fails on `case _: Array[?]` (tests/modules' readtasty-a):
// "ClassCastException: class dotty.tools.dotc.core.Types$CachedTermRef cannot be cast to class
// dotty.tools.dotc.core.Types$TypeRef", at TreeUnpickler.readSimpleType (TreeUnpickler.scala:494). The same on
// integration a097de53d9. Found by reviewer #15 of the workers exports helper (push 9).
package r3b
import r3a.*
@main def run(): Unit = println(Api.kind(1))
