// A given whose type is an abstract member of `this` (`b: X`, which teq's class index files under no class) keeps its
// place among `implicitMembers` (`computeMemberNames`, SymDenotations.scala:2409-2425), T2's before T1's for `object M
// extends T2, T1`; both are `Ops` as seen from M, so neither is preferred and scalac 3.8.4 names b before a. master
// (efddb38de3) and the head (61c1af7941) "both a and b provide pick on R" (the unindexed givens after the indexed
// ones); push 5 "both b and a", in its own report at the selection. Queued: push 6 (typeMismatch whole).
class R
trait Ops:
  extension (r: R) def pick: String
class Impl(n: String) extends Ops:
  extension (r: R) def pick: String = n
trait T2:
  type X <: Ops
  def mk: X
  given b: X = mk
trait T1:
  given a: Ops = Impl("a")
object M extends T2, T1:
  type X = Ops
  def mk: Ops = Impl("b")
  def run(): Unit = println((new R).pick)
object Main:
  def main(args: Array[String]): Unit = M.run()
