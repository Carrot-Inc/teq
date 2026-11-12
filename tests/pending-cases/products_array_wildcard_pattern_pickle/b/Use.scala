// A downstream of a/ as tests/modules builds it. scalac 3.8.4
// prints the .expected. scalac's -from-tasty read of teq's
// pickle of a/ is to read `case _: Array[?]`
// (tests/modules' readtasty-a). Found by reviewer #15
// of the workers exports helper (push 9).
package r3b
import r3a.*
@main def run(): Unit = println(Api.kind(1))
