// jars: scala-library typetest-lib
// std: scala-library
// targets: jvm
// The ABI of `scala.reflect.TypeTest` on the JVM: a synthesized instance and a handwritten one
// implement scala-library's interface, `unapply(Object): Option` once erased, with the bridge
// where an implementation specialises `S`; scalac-compiled code (tests/support/typetest_lib.scala,
// the jar typetest-lib) calls them through the interface.
import scala.reflect.TypeTest
import tt.{Animal, Cat, Dog, Lib}

object Main:
  val handwritten: TypeTest[Animal, Dog] = new TypeTest[Animal, Dog]:
    def unapply(x: Animal): Option[x.type & Dog] = x match
      case d: Dog if d.name.length > 1 => Some(d.asInstanceOf[x.type & Dog])
      case _ => None
  def main(args: Array[String]): Unit =
    println(Lib.viaInterface(summon[TypeTest[Any, String]], "ok"))
    println(Lib.viaInterface(summon[TypeTest[Any, String]], 1))
    println(Lib.defined[Any, Int](1)); println(Lib.defined[Any, Int]("s"))
    println(Lib.defined[Dog, Animal](null))
    println(Lib.narrowed(summon[TypeTest[Animal, Dog]], Dog("rex")))
    println(Lib.narrowed(summon[TypeTest[Animal, Dog]], Cat()))
    println(Lib.narrowed(handwritten, Dog("rex"))); println(Lib.narrowed(handwritten, Dog("r"))); println(Lib.narrowed(handwritten, Cat()))
    println(Lib.onString(summon[TypeTest[String, "ok"]], "ok")); println(Lib.onString(summon[TypeTest[String, "ok"]], "no"))
    println((Dog("rex"): Animal) match { case handwritten(d) => s"matched $d"; case _ => "unmatched" })
