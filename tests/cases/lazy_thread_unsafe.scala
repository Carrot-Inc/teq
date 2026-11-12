// `@threadUnsafe` lazy vals give the values scalac's give: computed once, a throwing initialiser run again, a member of
// a class, an object and a trait. scalac writes them with dotty's transformMemberDefThreadUnsafe (a typed field and a
// bitmap flag); teq writes a marked member over an unsynchronised `Object` holder, null until computed (the typed
// field and its flag are the queued part lazy-val-records), so the class files differ and the values do not. A
// recursive `@threadUnsafe` member recomputes as scalac's does (scala3_i1856, lazy_thread_unsafe_reentrant).
import scala.annotation.threadUnsafe

class Unsafe:
  var runs = 0
  @threadUnsafe lazy val text: String = { runs += 1; "unsafe" }
  @threadUnsafe lazy val number: Int = { runs += 1; 42 }
  @threadUnsafe lazy val nothingYet: String = null

class InitError extends Exception
class LazyThreadUnsafe(x: Int):
  private var init = false
  @threadUnsafe lazy val value: Int =
    if !init then
      init = true
      throw new InitError
    x

object Holder:
  var runs = 0
  @threadUnsafe lazy val v: Long = { runs += 1; 7L }

trait Mixed:
  var traitRuns = 0
  @threadUnsafe lazy val t: String = { traitRuns += 1; "from a trait" }
class Mixes extends Mixed

object Main:
  def main(args: Array[String]): Unit =
    val u = new Unsafe
    println(s"${u.text} ${u.text} ${u.number} ${u.number} ${u.nothingYet} runs ${u.runs}")
    val l = new LazyThreadUnsafe(5)
    println(try l.value catch case _: InitError => "InitError")
    println(l.value)
    println(s"${Holder.v} ${Holder.v} runs ${Holder.runs}")
    val m = new Mixes
    println(s"${m.t} ${m.t} runs ${m.traitRuns}")
