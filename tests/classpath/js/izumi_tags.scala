// jars: izumi-reflect izumi-reflect-boopickle scala-collection-compat
// targets: js interp
//> using dep dev.zio::izumi-reflect:3.0.9
// izumi-reflect's `Tag` from its jar: the macro's representations, the variances it reads from
// `declaredTypes` and `HKTypeLambda.typeParams`, higher-kinded tags built from type lambdas
// over context-bound parameters, and `<:<` over the inheritance database of a case class.
import izumi.reflect.Tag
import scala.collection.mutable.Builder

trait Greeter
trait Counter
final case class Settings(prefix: String)

object Main:
  def listOf[B: Tag]: Tag[List[B]] = Tag[List[B]]
  def eitherOf[B: Tag, C: Tag]: Tag[Either[B, C]] = Tag[Either[B, C]]
  def builderOf[B: Tag, C[+X] <: Iterable[X]](using t: Tag[C[B]]): Tag[Builder[B, C[B]]] = Tag[Builder[B, C[B]]]

  def main(args: Array[String]): Unit =
    println(Tag[Int].tag.shortName)
    println(Tag[List[String]].tag.repr)
    println(Tag[Greeter & Counter].tag.repr)
    println(Tag[Either[String, Option[Int]]].tag.shortName)
    println(Tag[Settings].tag <:< Tag[Product].tag)
    println(Tag[Settings].tag <:< Tag[Equals].tag)
    println(Tag[List[Int]].tag <:< Tag[Seq[Any]].tag)
    println(Tag[Greeter].tag =:= Tag[Counter].tag)
    println(listOf[Int].tag)
    println(eitherOf[Int, String].tag)
    println(builderOf[Int, List].tag)
    println(listOf[Int].tag =:= Tag[List[Int]].tag)
