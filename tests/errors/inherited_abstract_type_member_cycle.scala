// A class's inherited abstract type members that bound each
// other (`Zed <: Aye` from Z, `Aye <: Zed` from A) are a cycle:
// `checkNonCyclicInherited` (Checking.scala:496-519) walks
// the class's `abstractTypeMembers`, and scalac 3.8.4 reports
// "cyclic reference involving type Aye" at C. Queued: the owner of
// Checking.checkNonCyclicInherited and Type member enumeration.
trait Z { type Zed <: Aye; type Aye }
trait A { type Aye <: Zed; type Zed }
abstract class C extends A, Z
object Main { def main(args: Array[String]): Unit = () }
// expect: 9:16: error: cyclic reference involving type Aye
// expect: 1 error found
