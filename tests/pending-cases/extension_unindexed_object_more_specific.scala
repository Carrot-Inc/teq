// Of a given whose type is an abstract member of `this` (`b: X`, `X = Ops` in M) and a given object (`a`, of type
// `a.type <: Ops`), the object is the more specific (`compareAlternatives`, Implicits.scala; `isAsSpecific` of the two
// types as seen from M), so scalac 3.8.4 takes `a` and prints a. master (efddb38de3) and the head (61c1af7941) report
// "both a and b provide pick on R", and push 5 "both b and a": teq's comparison of the two candidates does not see `X`
// as `Ops` from M. Not push 5's order change. Queued: the implicits port, compareAlternatives over a member seen from
// its prefix.
class R
trait Ops:
  extension (r: R) def pick: String
trait T2:
  type X <: Ops
  def mk: X
  given b: X = mk
trait T1:
  given a: Ops with
    extension (r: R) def pick: String = "a"
object M extends T2, T1:
  type X = Ops
  def mk: Ops = new Ops { extension (r: R) def pick: String = "b" }
  def run(): Unit = println((new R).pick)
object Main:
  def main(args: Array[String]): Unit = M.run()
