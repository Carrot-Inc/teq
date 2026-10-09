// The tag search inside an inline definition (the evidence chosen at the definition, as scalac's
// `tryWithTypeTest` chooses it there, whatever the call site's argument), the applied abstract
// constructor's bound in the fallback (`F[Int]` under `F[X] <: Base` tests `Base`), and a
// `TypeTest` synthesized for a singleton type (the identity test).
import scala.reflect.{ClassTag, TypeTest}

trait Base
class Child extends Base:
  override def toString = "Child"
class Token

object Main:
  inline def matches[T](x: Any)(using ClassTag[T]): Boolean = x match
    case _: T => true
    case _ => false
  def forwarded[T: ClassTag](x: Any): Boolean = matches[T](x)
  inline def tested[T](x: Any)(using TypeTest[Any, T]): Boolean = x match
    case _: T => true
    case _ => false
  def deny[T]: TypeTest[Any, T] = new TypeTest[Any, T]:
    def unapply(x: Any): Option[x.type & T] = { println("denied"); None }
  def applied[F[X] <: Base](x: Any)(using a: TypeTest[Any, F[Int]], b: TypeTest[Any, F[Int]]): Boolean = x match
    case _: F[Int] @unchecked => true
    case _ => false
  def main(args: Array[String]): Unit =
    println(forwarded[String](1)); println(forwarded[String]("s")); println(matches[Int](1))
    println(tested[String]("ok")(using deny)); println(tested[String]("ok"))
    println(applied[[X] =>> Child]("wrong")(using deny, deny)); println(applied[[X] =>> Child](Child())(using deny, deny))
    println(applied[[X] =>> Child](null)(using deny, deny))
    val token = new Token
    val tt = summon[TypeTest[Any, token.type]]
    println(tt.unapply(token).isDefined); println(tt.unapply(new Token).isDefined); println(tt.unapply(null).isDefined)
    println((token: Any) match { case t: token.type => "same"; case _ => "other" })
