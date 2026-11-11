// A class's inherited abstract type members that bound each other (`Zed <: Aye` from Z, `Aye <: Zed` from A) are a
// cycle: `checkNonCyclicInherited` (Checking.scala:496-519) walks the class's `abstractTypeMembers`, and scalac 3.8.4
// reports "cyclic reference involving type Aye" at C. master (efddb38de3) accepts the program; the integration tip
// (075748ee7b), the head (61c1af7941) and push 5 run past 60 seconds on it: a TR regression from terminating to not
// terminating, not push 5's (the integration tip has neither push 4 nor 5). Queued: the owner of
// Checking.checkNonCyclicInherited and Type member enumeration (Codex's q15 on push 5).
trait Z { type Zed <: Aye; type Aye }
trait A { type Aye <: Zed; type Zed }
abstract class C extends A, Z
object Main { def main(args: Array[String]): Unit = () }
// expect: 9:16: error: cyclic reference involving type Aye
// expect: 1 error found
