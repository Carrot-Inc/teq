// jars: scala-library scala2-lib
// Mixin.traitInits needs arguments only for a mixin's parameter getters.
import scala2lib.indirect.Both

object Indirect extends Both
class IndirectClass extends Both
class Inherited extends IndirectClass

@main def run(): Unit =
  println(Indirect.left("object"))
  println(Indirect.right("object"))
  val instance = new Inherited
  println(instance.left("class"))
  println(instance.right("class"))
