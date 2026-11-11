// The implicit scope's eligible list is each companion reference's `implicitMembers` in turn (`OfTypeImplicits.refs`,
// Implicits.scala:279-282), and `sort` keeps it for candidates that tie (1648-1660: one level, arity 0, four base
// classes each): `object R extends T2, T1` lists T2's `b` before T1's `a`, so scalac 3.8.4 names b before a. master
// (efddb38de3) and the head (61c1af7941) "both a and b provide pick on R" (the list sorted by symbol); push 5 "both b
// and a", in its own report at the selection. Queued: push 6 (typeMismatch whole).
trait Ops:
  extension (r: R) def pick: String
trait T1:
  given a: Ops with
    extension (r: R) def pick: String = "a"
trait T2:
  given b: Ops with
    extension (r: R) def pick: String = "b"
class R
object R extends T2, T1
object Main:
  def main(args: Array[String]): Unit = println((new R).pick)
