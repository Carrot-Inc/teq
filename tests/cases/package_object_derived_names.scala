//> using dep com.softwaremill.magnolia1_3::magnolia:1.3.18
// jars: magnolia
// The definitions of a source `package object p` are members of the module `p.package` (dotty's
// `Desugar.packageModuleDef`), as a derivation and reflection see them: magnolia's `TypeInfo` of a case class and of a
// sealed trait with its subtypes (the owner `p.package`, its full and short names), a `Mirror`'s labels, and the runtime
// class names of an object, a case class and a case object (the classes `p.package$...`) with the `toString` of the
// latter two.
import magnolia1.*
import scala.deriving.Mirror
import scala.compiletime.{constValue, constValueTuple}

package object p:
  case class C(i: Int)
  sealed trait S
  case class S1() extends S
  case object S2 extends S
  object O

trait Names[T]:
  def lines: List[String]

object Names extends Derivation[Names]:
  given Names[Int] with
    def lines = Nil
  private def show(t: TypeInfo): String = s"owner=${t.owner} full=${t.full} short=${t.short}"
  def join[T](ctx: CaseClass[Names, T]): Names[T] = new Names[T]:
    def lines = List(s"case class ${show(ctx.typeInfo)}") ++ ctx.params.map(q => s"  param ${q.label}")
  def split[T](ctx: SealedTrait[Names, T]): Names[T] = new Names[T]:
    def lines = List(s"sealed trait ${show(ctx.typeInfo)}") ++ ctx.subtypes.map(s => s"  subtype ${show(s.typeInfo)}")

inline def labels[T](using m: Mirror.Of[T]): String =
  constValue[m.MirroredLabel] + " " + constValueTuple[m.MirroredElemLabels].toList.mkString("(", ",", ")")

object Main:
  def main(args: Array[String]): Unit =
    Names.derived[p.C].lines.foreach(println)
    Names.derived[p.S].lines.foreach(println)
    Names.derived[p.S1].lines.foreach(println)
    println(labels[p.C])
    println(labels[p.S])
    println(labels[p.S1])
    println(p.O.getClass.getName)
    println(p.C(1).getClass.getName)
    println(p.C(1).toString)
    println(p.S2.getClass.getName)
    println(p.S2.toString)
