// The ClassTag synthesis (`Synthesizer.synthesizedClassTag`): the tag of the erasure's class where the type symbol is
// no special class. A path or a refinement over an alias of `Any` is the alias's (`Object`), one over `Unit` is
// `ClassTag.Unit`; a union with `Unit` takes its box, whose name is the platform's (class_tag_synthesis_jvm prints the
// JVM's); an opaque type takes its underlying type's, `this.type` its self type's erasure, and a union of any number of
// parts its lub.
import scala.reflect.ClassTag
trait S
class C
trait SelfT { self: C =>
  def tag = Main.t[this.type]
}
object Al:
  type AA = Any
object Ids:
  opaque type Id = Int
  def of(n: Int): Id = n
object Main:
  type L33 = "k0" | "k1" | "k2" | "k3" | "k4" | "k5" | "k6" | "k7" | "k8" | "k9" | "k10" | "k11" | "k12" | "k13" | "k14" | "k15" | "k16" | "k17" | "k18" | "k19" | "k20" | "k21" | "k22" | "k23" | "k24" | "k25" | "k26" | "k27" | "k28" | "k29" | "k30" | "k31" | "k32"
  def t[T](using c: ClassTag[T]): String = s"$c ${c.runtimeClass.getName}"
  def main(args: Array[String]): Unit =
    val aa: Al.AA = 1
    val u: Unit = ()
    println(t[aa.type])
    println(t[Al.AA { def foo: Int }])
    println(t[u.type])
    println(summon[ClassTag[Null | Unit]].runtimeClass eq classOf[scala.runtime.BoxedUnit])
    println(summon[ClassTag[Unit & S]].runtimeClass eq classOf[scala.runtime.BoxedUnit])
    println((new C with SelfT).tag)
    println(Array.fill[Ids.Id](2)(Ids.of(7)).length)
    println(Array[L33]("k0", "k1").mkString)
