// teq: --wconf msg=cannot.be.checked.at.runtime:s
// A polymorphic function type is tested, cast and given a ClassTag by its erasure, the function class of its arity
// (`TypeErasure`'s `PolyFunctionOf` arm, `eraseRefinedFunctionApply`): a polymorphic function value passes the test
// and the cast, and an array of such values holds one.
import scala.reflect.ClassTag

object P:
  type Id = [A] => A => A
  def main(args: Array[String]): Unit =
    val id: Id = [A] => (x: A) => x
    val any: Any = id
    println(any.isInstanceOf[Id])
    println(any match { case _: Id => "poly"; case _ => "other" })
    println(any.asInstanceOf[Id](3))
    println(("x": Any).isInstanceOf[Id])
    println(summon[ClassTag[Id]])
    println(summon[ClassTag[Id | Null]])
    println(Array[Id](id).length)
    println(Array[Id | Null](id, null).length)
