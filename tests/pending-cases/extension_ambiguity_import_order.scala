// A wildcard import's implicits are `pre.implicitMembers` (ImportInfo.importedImplicits, ImportInfo.scala:131-134),
// over `memberNames` in the order `computeMemberNames` adds them (SymDenotations.scala:2409-2425): T2's `aaa` before
// T1's `zzz` for `object M extends T2, T1`, so scalac 3.8.4 names aaa before zzz. master (efddb38de3) and the head
// (61c1af7941) "both zzz and aaa provide pick on R" (memberNames in interned-name order); push 5 "both aaa and zzz", in
// its own report at the selection. Queued: push 6 (typeMismatch whole).
class R
trait Ops:
  extension (r: R) def pick: String
trait T1:
  given zzz: Ops with
    extension (r: R) def pick: String = "z"
trait T2:
  given aaa: Ops with
    extension (r: R) def pick: String = "a"
object M extends T2, T1
object Main:
  import M.given
  def main(args: Array[String]): Unit = println((new R).pick)
